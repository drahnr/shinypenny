use super::super::types::Pt;

pub(super) struct RenderState {
    pub(super) vstart: Pt,
    pub(super) vstep: Pt,
    pub(super) vcursor: Pt,

    pub(super) hbounds: Vec<Pt>,
    pub(super) hcursor: usize,
}

impl RenderState {
    pub(super) fn new<X>(
        vstart: impl Into<Pt>,
        vstep: impl Into<Pt>,
        hbounds: impl IntoIterator<IntoIter = X, Item = Pt>,
    ) -> Self
    where
        X: Iterator<Item = Pt>,
    {
        let hbounds: Vec<Pt> = hbounds.into_iter().collect::<Vec<_>>();
        assert!(hbounds.len() >= 2);

        let vstart = vstart.into();
        let vstep = vstep.into();

        Self {
            vstart,
            vstep,
            vcursor: vstart,
            hcursor: 0usize,
            hbounds,
        }
    }

    pub(super) fn advance_to_next_row(&mut self) {
        self.vcursor -= self.vstep;
    }
    pub(super) fn advance_v(&mut self, x: Pt) {
        self.vcursor -= x;
    }
    pub(super) fn advance_to_next_column(&mut self) {
        self.hcursor += 1;
    }

    pub(super) fn reset_column(&mut self) {
        self.hcursor = 0;
    }

    pub(super) fn current_column_left(&self) -> Pt {
        *self.hbounds.get(self.hcursor).expect("Left must exist")
    }
    pub(super) fn current_column_right(&self) -> Pt {
        *self
            .hbounds
            .get(self.hcursor + 1)
            .expect("Right must exist")
    }
    pub(super) fn current_column_x_range(&self) -> (Pt, Pt) {
        (self.current_column_left(), self.current_column_right())
    }
    pub(super) fn current_row_y_range(&self) -> (Pt, Pt) {
        (self.vcursor, self.vcursor - self.vstep)
    }
}
