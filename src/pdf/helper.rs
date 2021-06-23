use super::super::types::*;
use super::constants::{DIN_A4, TTF_BOLD, TTF_REGULAR};
use super::tabular::*;
use super::types::*;
use crate::errors::Result;

use printpdf::image::GenericImageView;
use printpdf::*;

/// Workaround for `printpdf::Document` being incomplete unless written to a buffer or disk.
fn flush_pdf_ops(doc: PdfDocumentReference) -> Result<lopdf::Document> {
    let mut buffer = Vec::with_capacity(4 << 20);
    {
        let mut buffw = std::io::BufWriter::new(&mut buffer);
        doc.save(&mut buffw)?;
    }

    Ok(lopdf::Document::load_mem(&buffer)?)
}

/// Compress the image a jpeg and include it appropriately.
fn create_jpg_image(
    active_layer: PdfLayerReference,
    anchor: Point,
    image: image::DynamicImage,
    scale: f64,
    dpi: f64,
) -> Result<()> {
    // whatever it was, store it as jpeg into the buffer
    let mut buffer = Vec::<u8>::with_capacity(4e6 as usize);
    image.write_to(&mut buffer, image::ImageOutputFormat::Jpeg(80))?;

    let xobj = {
        let dim = image.dimensions();
        let color_type = image.color();
        let color_bits = ColorBits::from(color_type);
        let color_space = ColorSpace::from(color_type);

        printpdf::ImageXObject {
            width: Px(dim.0 as usize),
            height: Px(dim.1 as usize),
            color_space,
            bits_per_component: color_bits,
            image_data: buffer.clone(),
            interpolate: true,
            image_filter: Some(ImageFilter::DCT),
            clipping_bbox: None,
        }
    };

    let image: printpdf::Image = Image::from(xobj);

    image.add_to_layer(
        active_layer,
        Some(anchor.x.into()),
        Some(anchor.y.into()),
        None,
        Some(scale),
        Some(scale),
        Some(dpi),
    );
    Ok(())
}

/// Introduce a sized image page.
pub fn sized_image_page(image: image::DynamicImage) -> Result<lopdf::Document> {
    let (width, height) = image.dimensions();
    let dpi = 200. as f64;

    let height: Mm = Px(height as usize).into_pt(dpi).into();
    let width: Mm = Px(width as usize).into_pt(dpi).into();

    let scale: f64 = DIN_A4.width / width;

    let allowed: std::ops::Range<f64> = 0.25..4.;
    if !allowed.contains(&scale) {
        log::warn!(
            "Clamped scale factor of {:.02} to {:.02}..{:.02}",
            scale,
            allowed.start,
            allowed.end
        );
    }
    let scale = scale.max(allowed.start).min(allowed.end);

    let dim = Dimensions { height, width };
    let (document, page1, layer1) = PdfDocument::new(
        "Separation",
        dim.width * scale,
        dim.height * scale,
        "Layer 1",
    );
    let document = document.with_conformance(PdfConformance::Custom(CustomPdfConformance {
        requires_icc_profile: false,
        requires_xmp_metadata: false,
        ..Default::default()
    }));
    let active_layer = document.get_page(page1).get_layer(layer1);

    create_jpg_image(
        active_layer,
        Point {
            x: Pt(0.),
            y: Pt(0.),
        },
        image,
        scale,
        dpi,
    )?;

    flush_pdf_ops(document)
}

/// Add an image with an anchor point to a given layer.
pub fn add_image(
    active_layer: PdfLayerReference,
    anchor: Point,
    image: printpdf::image::DynamicImage,
    align: Alignment,
) -> Result<()> {
    let dpi = 300f64;
    let (width, height) = image.dimensions();
    let width = Px(width as usize).into_pt(dpi);
    let height = Px(height as usize).into_pt(dpi);

    let scale = Pt::from(DIN_A4.height) * 0.19 / height;

    let x = match align {
        Alignment::Left => anchor.x,
        Alignment::Right => anchor.x - width * scale,
        Alignment::Center => anchor.x - width * scale / 2.0f64,
    };
    let anchor = Point { x, y: anchor.y };
    create_jpg_image(active_layer, anchor, image, scale, dpi)?;

    Ok(())
}

pub fn separation_page(desc: &str) -> Result<lopdf::Document> {
    let dim = (Mm(20 as f64), DIN_A4.width);
    let (doc, page1, layer1) = PdfDocument::new("Separation", dim.1, dim.0, "Layer 1");
    let active_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc.add_external_font(TTF_REGULAR)?;

    let white = Color::Rgb(Rgb {
        r: 1.,
        g: 1.,
        b: 1.,
        icc_profile: None,
    });
    let gray = Color::Greyscale(Greyscale {
        percent: 0.70,
        icc_profile: None,
    });

    let left = Pt(0.);
    let bottom = Pt(0.);
    let top = dim.0.into();
    let right = dim.1.into();
    let line = Line {
        points: vec![
            (Point { x: left, y: top }, false),
            (Point { x: left, y: bottom }, false),
            (
                Point {
                    x: right,
                    y: bottom,
                },
                false,
            ),
            (Point { x: right, y: top }, false),
        ],
        is_closed: true,
        has_fill: true,
        has_stroke: false,
        is_clipping_path: false,
    };

    let background = gray.clone();
    let foreground = white.clone();

    active_layer.save_graphics_state();
    active_layer.save_graphics_state();
    active_layer.set_fill_color(background);

    active_layer.add_shape(line);

    active_layer.restore_graphics_state();

    let anchor = Point {
        x: Pt::from(dim.1) * 0.50,
        y: Pt::from(dim.0) * 0.20,
    };

    text(&active_layer, anchor, desc, &font, 20, Alignment::Center)?;

    active_layer.set_fill_color(foreground);
    active_layer.restore_graphics_state();

    flush_pdf_ops(doc)
}

pub fn tabular(
    bankinfo: BankInfo,
    company: CompanyInfo,
    rows: &[Row],
    totals: Totals,
    learning_budget: bool,
) -> Result<lopdf::Document> {
    let (doc, page1, layer1) =
        PdfDocument::new("Reimbursement", DIN_A4.width, DIN_A4.height, "Layer 1");
    let active_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc.add_external_font(TTF_REGULAR)?;
    let bold = doc.add_external_font(TTF_BOLD)?;

    let black = Color::Rgb(Rgb {
        r: 0.,
        g: 0.,
        b: 0.,
        icc_profile: None,
    });
    let white = Color::Rgb(Rgb {
        r: 1.,
        g: 1.,
        b: 1.,
        icc_profile: None,
    });
    let darkgray = Color::Greyscale(Greyscale {
        percent: 0.50,
        icc_profile: None,
    });
    let gray = Color::Greyscale(Greyscale {
        percent: 0.70,
        icc_profile: None,
    });

    let style0 = RenderStyle {
        font: font.clone(),
        size: 11,
        foreground: black.clone(),
        background: gray.clone(),
        alignment: Alignment::Right,
    };

    let style1 = RenderStyle {
        font: font.clone(),
        size: 11,
        foreground: black.clone(),
        background: gray.clone(),
        alignment: Alignment::Right,
    };

    let style2 = RenderStyle {
        font: bold,
        size: 12,
        foreground: black.clone(),
        background: white.clone(),
        alignment: Alignment::Right,
    };

    let styleset = RenderStyleSet {
        header: style0.clone(),
        data: style1.clone(),
        sum: style2.clone(),
    };

    let mut headers = vec![
        "Date",
        "Company",
        "Description",
        "Netto €",
        // insert tax levels here
        "Brutto €",
    ]
    .into_iter()
    .map(|x| x.to_owned())
    .collect::<Vec<_>>();

    let mut columns = ColumnWidthSet(vec![
        ColumnWidth(Mm(22.).into()),  // date
        ColumnWidth(Mm(40.).into()),  // company
        ColumnWidth(Mm(60.).into()),  // description
        ColumnWidth(Mm(45.).into()), // netto
        // insert tax ones here
        ColumnWidth(Mm(45.).into()), // brutto
    ]);

    use itertools::Itertools;

    // add column for each tax percentage, lowest first
    for percentage in totals
        .tax_total
        .keys()
        .sorted_by(|p1, p2| p1.cmp(&p2))
        .rev()
    {
        columns.0.insert(4, ColumnWidth(Mm(14.).into()));
        headers.insert(4, format!("{} %", percentage));
    }

    let total_width = columns.total_width();

    {
        let anchor = Point {
            x: Pt::from(DIN_A4.width) * 0.50,
            y: Pt::from(DIN_A4.height) * 0.83,
        };

        if let Some(image) = company.image {
            add_image(active_layer.clone(), anchor, image, Alignment::Center)?;
        }

        let anchor = Point {
            x: Pt::from(DIN_A4.width) * 0.50,
            y: Pt::from(DIN_A4.height) * 0.80,
        };

        text(
            &active_layer,
            anchor,
            "Application for reimbursement of expenses",
            &font,
            style1.size * 5 / 3,
            Alignment::Center,
        )?;
    }

    {
        let x = {
            let a = Pt::from(DIN_A4.width);
            if a > total_width {
                (a - total_width) / 2.
            } else {
                Pt(0.)
            }
        };

        let anchor = Point {
            x,
            y: Pt::from(DIN_A4.height) * 0.68,
        };
        let expenses = SummableTabular::new(
            &active_layer,
            anchor,
            headers.iter().map(|x| x.as_str()).collect::<Vec<&'_ str>>(),
            rows,
            Some(totals.into_iter()),
        );
        expenses.render(&styleset, columns)?;
    }

    {
        const HEADER: &'static [&'static str] =
            &["Name", "Institute", "IBAN", "BIC", "Reimbursement"];

        let rows = vec![
            bankinfo.name.clone(),
            bankinfo.institute().unwrap_or("".to_owned()),
            bankinfo.iban.to_string(), // adds a couple of spaces compared to `.electronic_str().to_owned()`
            bankinfo.bic().unwrap_or("".to_owned()),
            format!("{} €", totals.brutto),
        ];

        let mut anchor = Point {
            x: Pt::from(DIN_A4.width) * 0.25,
            y: Pt::from(DIN_A4.height) * 0.25,
        };
        for (item, value) in HEADER.iter().zip(rows) {
            text(
                &active_layer,
                anchor,
                *item,
                &style1.font,
                style1.size,
                Alignment::Right,
            )?;
            anchor.x += Pt(10.0);
            text(
                &active_layer,
                anchor,
                &value,
                &style2.font,
                style2.size,
                Alignment::Left,
            )?;
            anchor.x -= Pt(10.0);
            anchor.y -= Pt(20.0);
        }
    }

    {
        let mut anchor = Point {
            x: Pt::from(DIN_A4.width) * 0.10,
            y: Pt::from(DIN_A4.height) * 0.75,
        };

        const EMPLOYEE: &str = "Employee:";
        let width = text_width(EMPLOYEE, TTF_REGULAR, style1.size)?;

        text(
            &active_layer,
            anchor,
            EMPLOYEE,
            &style1.font,
            style1.size,
            Alignment::Left,
        )?;

        anchor.x += width + Pt(10.);
        text(
            &active_layer,
            anchor,
            &bankinfo.name,
            &style2.font,
            style2.size,
            Alignment::Left,
        )?;
    }

    {
        let mut anchor = Point {
            x: Pt::from(DIN_A4.width) * 0.50,
            y: Pt::from(DIN_A4.height) * 0.75,
        };

        const DATE: &str = "Date:";
        let width = text_width(DATE, TTF_REGULAR, style1.size)?;

        text(
            &active_layer,
            anchor,
            DATE,
            &style1.font,
            style1.size,
            Alignment::Left,
        )?;

        let now = chrono::Local::today();

        anchor.x += width + Pt(10.);
        text(
            &active_layer,
            anchor,
            &now.format("%Y-%m-%d").to_string(),
            &style2.font,
            style2.size,
            Alignment::Left,
        )?;
    }

    {
        let mut anchor = Point {
            x: Pt::from(DIN_A4.width) * 0.10,
            y: Pt::from(DIN_A4.height) * 0.70,
        };

        const EMPLOYEE: &str = "Learning Budget:";
        let width = text_width(EMPLOYEE, TTF_REGULAR, style1.size)?;

        text(
            &active_layer,
            anchor,
            EMPLOYEE,
            &style1.font,
            style1.size,
            Alignment::Left,
        )?;

        let content = if learning_budget { "YES" } else { "NO" };
        anchor.x += width + Pt(10.);
        text(
            &active_layer,
            anchor,
            content,
            &style2.font,
            style2.size,
            Alignment::Left,
        )?;
    }

    // footer
    if !company.name.is_empty() || !company.address.is_empty() {
        let y = Pt(30.);
        let mut anchor = Point { x: Pt(30.), y: y };

        let line = Line {
            points: vec![
                (Point { x: Pt(0.), y }, false),
                (
                    Point {
                        x: DIN_A4.width.into(),
                        y,
                    },
                    false,
                ),
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };

        let foreground = darkgray;

        active_layer.save_graphics_state();
        active_layer.set_fill_color(foreground.clone());
        active_layer.set_outline_color(foreground.clone());

        active_layer.add_shape(line);

        anchor.y -= Pt(10.);

        if !company.name.is_empty() {
            text(
                &active_layer,
                anchor,
                &company.name,
                &style2.font,
                style2.size * 3 / 4,
                Alignment::Left,
            )?;
        }

        anchor.y -= Pt(10.);
        if !company.address.is_empty() {
            text(
                &active_layer,
                anchor,
                &company.address,
                &style2.font,
                style2.size * 3 / 4,
                Alignment::Left,
            )?;
        }

        active_layer.restore_graphics_state();
    }

    flush_pdf_ops(doc)
}
