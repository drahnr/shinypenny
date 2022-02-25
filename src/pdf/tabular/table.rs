//! Craft a table based on nested iterators
//!
//! Assumes your values implements `trait ToString`.

use super::super::constants::TTF_REGULAR;
use super::text::text_width;
use super::ColumnWidthSet;
use super::{Alignment, RenderState, RenderStyle, RenderStyleSet};
use crate::errors::*;

use printpdf::*;

use std::marker::PhantomData;

pub struct SummableTabular<'a, 'b, T0, I0, II0, I1, II1, ST, SI, SV> {
    anchor: Point,
    header: Vec<&'a str>,
    content: T0,
    sum: Option<ST>,
    active_layer: &'b PdfLayerReference,
    _phantom1: PhantomData<I0>,
    _phantom2: PhantomData<II0>,
    _phantom3: PhantomData<I1>,
    _phantom4: PhantomData<II1>,
    _phantom5: PhantomData<SI>,
    _phantom6: PhantomData<SV>,
}

impl<'a, 'b, T0, I0, II0, I1, II1, ST, SI, SV>
    SummableTabular<'a, 'b, T0, I0, II0, I1, II1, ST, SI, SV>
where
    T0: IntoIterator<IntoIter = II0, Item = I0> + Clone,
    II0: Iterator<Item = I0>,
    I0: IntoIterator<IntoIter = II1, Item = I1>,
    II1: Iterator<Item = I1>,
    I1: ToString,

    ST: IntoIterator<IntoIter = SI, Item = SV> + Clone,
    SI: Iterator<Item = SV>,
    SV: ToString,
{
    pub fn new(
        active_layer: &'b PdfLayerReference,
        anchor: Point,
        header: Vec<&'a str>,
        content: T0,
        sum: Option<ST>,
    ) -> Self {
        Self {
            anchor,
            header,
            content,
            sum,
            active_layer,
            _phantom1: Default::default(),
            _phantom2: Default::default(),
            _phantom3: Default::default(),
            _phantom4: Default::default(),
            _phantom5: Default::default(),
            _phantom6: Default::default(),
        }
    }

    pub fn render(mut self, styleset: &RenderStyleSet, columnwidths: ColumnWidthSet) -> Result<()> {
        let mut hbounds = Vec::with_capacity(columnwidths.len() + 1);
        hbounds.insert(0, self.anchor.x);
        assert_eq!(hbounds.len(), 1);
        let mut x = self.anchor.x;
        hbounds.extend(columnwidths.into_iter().enumerate().map(|(idx, val)| {
            let _ = idx;
            x += val;
            x
        }));

        assert!(hbounds.len().saturating_sub(1) == self.header.clone().into_iter().count());

        let row_height = Pt(20.);
        let mut state = RenderState::new(self.anchor.y, row_height, hbounds);

        // draw headers
        self.header(&mut state, &styleset.header)?;

        // draw content
        for row in self.content.clone() {
            self.render_row(row, &mut state, &styleset.data)?;
            state.advance_to_next_row();
        }

        // draw columns
        state.reset_column();
        let iter = state.hbounds.clone().into_iter();
        for _ in iter {
            self.vline(&state)?;
            state.advance_to_next_column();
        }

        self.hline(&state)?;

        // draw bottom sum line
        if let Some(summed) = self.sum.clone() {
            state.reset_column();
            for (idx, sum) in summed.into_iter().enumerate() {
                let sum = sum.to_string();
                log::trace!("Total column {} with a sum value of {}", idx, sum);
                let xrange = state.current_column_x_range();
                self.render_cell(sum, xrange, &state, &styleset.sum)?;
                state.advance_to_next_column();
            }
            // finally bottom line
            state.advance_to_next_row();
            self.hline(&state)?;
            state.advance_v(Pt(2.));
            self.hline(&state)?;
        }

        Ok(())
    }

    fn render_row(&mut self, row: I0, state: &mut RenderState, style: &RenderStyle) -> Result<()> {
        state.reset_column();
        self.hline(state)?;
        for val in row {
            let xrange = state.current_column_x_range();
            self.render_cell(val.to_string(), xrange, state, style)?;
            state.advance_to_next_column();
        }
        Ok(())
    }

    fn render_cell(
        &mut self,
        val: impl Into<String>,
        xrange: (Pt, Pt),
        state: &RenderState,
        style: &RenderStyle,
    ) -> Result<()> {
        let text = val.into();
        if !text.is_empty() {
            let length = text_width(&text, TTF_REGULAR, style.size)?;

            let (left, right) = xrange;
            let x: Pt = match style.alignment {
                Alignment::Left => left,
                Alignment::Right => right - length,
                Alignment::Center => (left + right - length) / 2.0f64,
            };

            if left > x {
                log::warn!("Detected overlap due to overly long text >{}<", text);
            }
            if (left + length) > right {
                log::warn!("Detected overlap due to overly long text >{}<", text);
            }

            let anchor = Point {
                x,
                y: state.vcursor - state.vstep * 0.87, // measured from the bottom left
            };

            self.active_layer.use_text(
                text,
                style.size.into(),
                Mm::from(anchor.x),
                Mm::from(anchor.y),
                &style.font,
            );
        }

        Ok(())
    }

    fn header(&mut self, state: &mut RenderState, style: &RenderStyle) -> Result<()> {
        let (top, bottom) = state.current_row_y_range();
        let left = state
            .hbounds
            .first()
            .copied()
            .ok_or_else(|| eyre!("Failed to obtain first"))?;
        let right = state
            .hbounds
            .last()
            .copied()
            .ok_or_else(|| eyre!("Failed to obtain last"))?;
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

        self.active_layer.save_graphics_state();
        {
            self.active_layer.save_graphics_state();
            self.active_layer.set_fill_color(style.background.clone());

            self.active_layer.add_shape(line);

            self.active_layer.restore_graphics_state();
            self.active_layer.set_fill_color(style.foreground.clone());
        }

        self.hline(&*state)?;
        state.reset_column();
        for header_val in self.header.clone() {
            let xrange = state.current_column_x_range();
            self.render_cell(header_val, xrange, &*state, style)?;
            state.advance_to_next_column();
        }
        state.advance_to_next_row();

        self.hline(&*state)?; // make bottom line
        state.advance_v(Pt(1.0));
        self.hline(&*state)?; // and make it thick

        self.active_layer.restore_graphics_state();
        Ok(())
    }

    // fn double_hline(&self, from: Point, to: Point, delta: impl Into<Pt>) -> Result<()> {
    //     self.hline(from, to)?;
    //     let delta = delta.into();
    //     let from    = Point{ x: from.x, y: from.y + delta};
    //     let to    = Point{ x: to.x, y: to.y + delta};
    //     self.hline(from, to)
    // }

    fn hline(&mut self, state: &RenderState) -> Result<()> {
        let from = Point {
            y: state.vcursor,
            x: *state.hbounds.first().unwrap(),
        };
        let to = Point {
            y: state.vcursor,
            x: *state.hbounds.last().unwrap(),
        };
        self.line(from, to)
    }

    fn vline(&mut self, state: &RenderState) -> Result<()> {
        let from = Point {
            y: state.vstart,
            x: state.current_column_left(),
        };
        let to = Point {
            y: state.vcursor,
            x: state.current_column_left(),
        };
        self.line(from, to)
    }

    fn line(&mut self, from: Point, to: Point) -> Result<()> {
        let line = Line {
            points: vec![(from, false), (to, false)],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };

        self.active_layer.add_shape(line);
        Ok(())
    }
}
