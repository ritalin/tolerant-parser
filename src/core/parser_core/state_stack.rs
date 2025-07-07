use cactus::Cactus;

#[derive(PartialEq, Clone, Debug)]
pub struct StateStack {
    initial_state: usize,
    stack: Cactus<usize>,
    checkpoint: Cactus<(usize, bool)>,
}

impl StateStack {
    pub fn new(initial_state: usize) -> Self {
        Self { 
            initial_state,
            stack: Cactus::new().child(initial_state),
            checkpoint: Cactus::new(),
        }
    }

    pub fn peek_state(&self) -> Option<&usize> {
        self.stack.val()
    }

    pub fn push_state(&mut self, state: usize) {
        self.stack = self.stack.child(state);
    }

    pub fn pop_n_state(&mut self, mut pop_count: usize) -> Option<&usize> {
        while pop_count > 0 {
            let Some(parent) = self.stack.parent() else { break };
            self.stack = parent;
            pop_count -= 1;
        }

        match pop_count {
            0 => self.peek_state(),
            _ => None,
        }
    }

    pub fn pop_all(&mut self) {
        self.pop_n_state(self.stack.len());
    }

    pub fn reset(&mut self) {
        self.stack = Cactus::new().child(self.initial_state);
        self.checkpoint = Cactus::new();
    }

    #[inline]
    pub fn initial_state(&self) -> usize {
        self.initial_state
    }

    pub fn state_values(&self) -> Vec<usize> {
        self.stack.vals().cloned().collect()
    }

    pub fn mark_checkpoint(&mut self, state: usize, is_shift: bool) -> usize {
        self.checkpoint = self.checkpoint.child((state, is_shift));
        state
    }
    pub fn resolve_checkpoint(&mut self, mut pop_count: usize) -> Option<usize> {
        if pop_count == 0 {
            return None;
        }

        while pop_count > 1 {
            self.checkpoint = self.checkpoint.parent().unwrap_or_default();
            pop_count -= 1;
        }

        match self.checkpoint.val() {
            Some((state, is_shift)) if *is_shift => {
                // Use the top of the stack (i.e., the last shift) as the resume point.
                Some(*state)
            }
            Some(_) => {
                // Use the first state after a shift as the resume point (do not pop it).
                self.checkpoint.vals()
                .take_while(|(_, is_shift)| !is_shift)
                .map(|(state, _)| state)
                .last()
                .cloned()   
            }
            None => None
        }
    }
}
