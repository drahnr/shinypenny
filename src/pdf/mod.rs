use crate::errors::*;

use std::collections::BTreeMap;

use lopdf::{Document, Object, ObjectId};

use printpdf::*;

mod types;

pub mod constants;

mod tabular;

mod helper;
pub use self::helper::*;

use std::io::{BufRead, Read, Seek, SeekFrom};
use std::path::Path;

use fs_err as fs;

fn load_image(stream: impl BufRead + Seek, _ext: String) -> Result<Document> {
    let reader = image::io::Reader::new(stream);
    let reader = image::io::Reader::with_guessed_format(reader)?;
    let image = reader.decode()?;

    sized_image_page(image)
}

use infer::Infer;

fn load_pdf(path: &Path, buffered: impl BufRead) -> Result<lopdf::Document> {
    let document = lopdf::Document::load_from(buffered)
        .map_err(|e| eyre!("Could not open receipt {}: {:?}", path.display(), e))?;
    Ok(document)
}

pub fn load_receipt(path: impl AsRef<Path>) -> Result<lopdf::Document> {
    let path = path.as_ref();
    let f = fs::File::open(&path)?;

    let mut buffered = std::io::BufReader::new(f);
    let mut magic = vec![0u8; 16];
    buffered.read_exact(&mut magic).wrap_err_with(|| {
        eyre!(
            "File {} is too short to determine file type",
            path.display()
        )
    })?;

    // go back to the beginning
    buffered.seek(SeekFrom::Start(0))?;

    let infer = Infer::new();

    if let Some(detected) = infer.get(&magic) {
        log::info!(
            "Inferring by magic based mime type {}",
            detected.mime_type()
        );
        let document = match detected.mime_type() {
            mime if mime.starts_with("image/") => {
                load_image(buffered, detected.extension().to_owned())?
            }
            "application/pdf" => load_pdf(path, buffered)?,
            mime => bail!("Can not hande {} mime type of {}", mime, path.display()),
        };
        Ok(document)
    } else if let Some(ext) = path.extension().map(|x| x.to_string_lossy()) {
        log::warn!("Could not infer mime type from initial 16 bytes, fallback to file extension");
        let document = match ext.as_ref() {
            "png" | "jpeg" | "jpg" | "webp" | "bmp" => {
                load_image(buffered, ext.as_ref().to_owned())?
            }
            "pdf" => load_pdf(path, buffered)?,
            mime => bail!("Can not hande {} mime type of {}", mime, path.display()),
        };
        Ok(document)
    } else {
        bail!("Failed to determine file type of {}", path.display());
    }
}

/// Combine multiple pdf files into one.
pub fn combine(documents: &mut [Document]) -> Result<Document> {
    // Define a starting max_id (will be used as start index for object_ids)
    let mut max_id = 1;

    // Collect all Documents Objects grouped by a map
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();

    for (idx, document) in documents.into_iter().enumerate() {
        log::info!("Adding pdf {:02}", idx);
        document.renumber_objects_with(max_id);

        log::debug!("{:02} Contains", document.max_id.saturating_sub(max_id));
        max_id = document.max_id + 1;

        let pages = document.get_pages();
        let pages = pages
            .into_iter()
            .map(|(_, object_id)| {
                (
                    object_id,
                    document.get_object(object_id).unwrap().to_owned(),
                )
            })
            .collect::<BTreeMap<ObjectId, Object>>();

        documents_pages.extend(pages);
        documents_objects.extend(document.objects.clone());
    }

    // Initialize a new empty document
    let mut document = Document::with_version("1.5");

    // Catalog and Pages are mandatory
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Process all objects except "Page" type
    for (object_id, object) in documents_objects.iter() {
        // We have to ignore "Page" (as are processed later), "Outlines" and "Outline" objects
        // All other objects should be collected and inserted into the main Document
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                log::info!("Adding catalog {:?}", &object);
                // Collect a first "Catalog" object and use it for the future "Pages"
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            "Pages" => {
                // Collect and update a first "Pages" object and use it for the future "Catalog"
                // We have also to merge all dictionaries of the old and the new "Pages" object
                if let Ok(dictionary) = object.as_dict() {
                    log::info!("Adding pages {:?}", &dictionary);
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }

                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => {} // Ignored, processed later and separately
            "Outlines" => {
                // Ignored, not supported yet
                log::warn!("Dropping outlines");
            }
            "Outline" => {
                // Ignored, not supported yet
                log::warn!("Dropping outlines");
            }
            x => {
                log::info!("Adding other object {} {:?}", x, &object);
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    let pages_object = if let Some(pages_object) = pages_object {
        pages_object
    } else {
        bail!("No pages found in document.")
    };

    // Iter over all "Page" and collect with the parent "Pages" created before
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            log::info!("Adding dictionary {:?}", &dictionary);
            dictionary.set("Parent", pages_object.0);

            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // If no "Catalog" found abort
    let catalog_object = if let Some(catalog_object) = catalog_object {
        catalog_object
    } else {
        bail!("Catalog root not found.");
    };

    // Build a new "Pages" with updated fields
    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();

        // Set new pages count
        dictionary.set("Count", documents_pages.len() as u32);

        // Set new "Kids" list (collected from documents pages) for "Pages"
        dictionary.set(
            "Kids",
            documents_pages
                .into_iter()
                .map(|(object_id, _)| Object::Reference(object_id))
                .collect::<Vec<_>>(),
        );

        document
            .objects
            .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    // Build a new "Catalog" with updated fields
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs

        document
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    // Update the max internal ID as wasn't updated before due to direct objects insertion
    document.max_id = document.objects.len() as u32;

    // Reorder all new Document objects
    document.renumber_objects();
    document.compress();

    // Save the merged PDF
    Ok(document)
}
