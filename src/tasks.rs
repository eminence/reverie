
use std::io::{Result};
use std::collections::HashMap;
use nix::unistd::Pid;

use crate::remote::*;

/*
lazy_static! {
    static ref TRACED_TASKS: HashMap<Pid, TracedTask<'static>> = {
        HashMap::new()
    };
}
*/

pub struct TracedTasks<'a>{
    tasks: HashMap<Pid, &'a mut TracedTask>,
}

impl <'a> TracedTasks<'a>{
    pub fn new() -> Self {
        TracedTasks{tasks: HashMap::new()}
    }
    pub fn add(&mut self, task: &'a mut TracedTask) -> Result<()> {
        self.tasks.insert(task.pid, task);
        Ok(())
    }
    pub fn remove(&mut self, pid: Pid) -> Result<()> {
        self.tasks.remove(&pid);
        Ok(())
    }
    pub fn get(&self, pid: Pid) -> &TracedTask {
        self.tasks.get(&pid).unwrap()
    }
    pub fn get_mut(&mut self, pid: Pid) -> &mut TracedTask {
        self.tasks.get_mut(&pid).unwrap()
    }
}