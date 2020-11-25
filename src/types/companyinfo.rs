use super::*;

#[derive(Clone)]
pub(crate) struct CompanyInfo {
    pub(crate) image: Option<printpdf::image::DynamicImage>,
    pub(crate) name: String,
    pub(crate) address: String,
}

impl CompanyInfo {
    pub(crate) fn new(name: &str, address: &str, image_path: Option<PathBuf>) -> Result<Self> {
        let image = if let Some(image_path) = image_path {
            log::trace!("Loading company image from {}", image_path.display());
            let file = fs::OpenOptions::new().read(true).open(&image_path)?;
            let reader = std::io::BufReader::with_capacity(2048, file);
            let reader = printpdf::image::io::Reader::new(reader).with_guessed_format()?;
            log::trace!("Determined company image format: {:?}", reader.format());
            let image = reader.decode()?;
            Some(image)
        } else {
            None
        };
        let name = name.to_owned();
        let address = address.to_owned();
        Ok(Self {
            image,
            name,
            address,
        })
    }
}
