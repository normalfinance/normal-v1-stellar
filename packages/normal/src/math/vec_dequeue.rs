use soroban_sdk::{contracttype, vec, Env, Vec};

#[contracttype]
#[derive(Clone)]
pub struct VecDeque {
    data: Vec<u32>,
}

impl VecDeque {
    pub fn new(env: &Env) -> Self {
        Self {
            data: Vec::new(env),
        }
    }

    // Push to the back
    pub fn push_back(&mut self, env: &Env, value: u32) {
        self.data.append(&vec![env, value]);
    }

    // Push to the front (prepend)
    pub fn push_front(&mut self, env: &Env, value: u32) {
        let mut new_data = Vec::new(env);
        new_data.append(&vec![env, value]);
        for item in self.data.iter() {
            new_data.append(&vec![env, item]);
        }
        self.data = new_data;
    }

    // Pop from the back
    pub fn pop_back(&mut self) {
        // if !self.data.is_empty() {
        //     let last_index = self.data.len() - 1;
        //     let new_data = self.data.slice(0..last_index);
        //     let last_element = self.data.get(last_index).unwrap();
        //     (Self { data: new_data }, Some(last_element));
        // } else {
        //     self.clone();
        // }
    }

    // Pop from the front
    pub fn pop_front(&mut self) {
        self.data.remove(0);
        // if self.data.len() > 0 { Some(self.data.remove(0)) } else { None }
    }
}
