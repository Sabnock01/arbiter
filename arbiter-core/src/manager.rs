#![warn(missing_docs)]
#![warn(unsafe_code)]
//! Simulation managers are used to manage the environments for a simulation.
//! Managers are responsible for adding agents, running agents, deploying contracts, calling contracts, and reading logs.

use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};

use crate::{
    agent::{Agent, NotAttached},
    environment::{AtomicState, Environment, State},
};

/// Manages simulations.
#[derive(Default)]
pub struct Manager {
    /// The list of [`SimulationEnvironment`] that the simulation manager controls.
    pub environments: HashMap<String, Environment>,
    handles_and_states: HashMap<String, (std::thread::JoinHandle<()>, Arc<AtomicState>)>,
}

impl Manager {
    /// Constructor function to instantiate a [`SimulationManager`].
    pub fn new() -> Self {
        Self {
            environments: HashMap::new(),
            handles_and_states: HashMap::new(),
        }
    }

    /// Adds an environment to the [`SimulationManager`]'s list.
    pub fn add_environment<S: Into<String> + Clone>(
        &mut self,
        environment_label: S,
        block_rate: f64,
        seed: u64,
    ) -> Result<()> {
        if self
            .environments
            .get(&environment_label.clone().into())
            .is_some()
        {
            return Err(anyhow!("Environment already exists."));
        }
        self.environments.insert(
            environment_label.clone().into(),
            Environment::new(environment_label, block_rate, seed),
        );
        Ok(())
    }

    /// adds an agent to an environment
    pub fn add_agent(
        &mut self,
        agent: Agent<NotAttached>,
        environment_label: String,
    ) -> Result<()> {
        match self.environments.get_mut(&environment_label) {
            Some(environment) => {
                environment.add_agent(agent);
                Ok(())
            }
            None => Err(anyhow!("Environment does not exist.")),
        }
    }

    /// Runs an environment that is in the [`SimulationManager`]'s list.
    pub fn start_environment<S: Into<String> + Clone>(
        &mut self,
        environment_label: S,
    ) -> Result<()> {
        match self.environments.get_mut(&environment_label.clone().into()) {
            Some(environment) => match environment.state.load(std::sync::atomic::Ordering::Relaxed)
            {
                State::Initialization => {
                    let handle = environment.run();
                    self.handles_and_states.insert(
                        environment_label.into(),
                        (handle, environment.state.clone()),
                    );
                    Ok(())
                }
                State::Paused => {
                    environment
                        .state
                        .store(State::Running, std::sync::atomic::Ordering::Relaxed);
                    let (lock, pausevar) = &*environment.pausevar;
                    let _guard = lock.lock().unwrap();
                    pausevar.notify_all();
                    Ok(())
                }
                State::Running => Err(anyhow!("Environment is already running.")),
                State::Stopped => Err(anyhow!("Environment is stopped and cannot be restarted.")),
            },
            None => Err(anyhow!("Environment does not exist.")),
        }
    }

    pub fn pause_environment<S: Into<String> + Clone>(
        &mut self,
        environment_label: S,
    ) -> Result<()> {
        match self.environments.get_mut(&environment_label.clone().into()) {
            Some(environment) => match environment.state.load(std::sync::atomic::Ordering::Relaxed)
            {
                State::Initialization => Err(anyhow!("Environment is not running.")),
                State::Running => {
                    environment
                        .state
                        .store(State::Paused, std::sync::atomic::Ordering::Relaxed);
                    println!("Changed state to paused.");
                    Ok(())
                }
                State::Paused => Err(anyhow!("Environment is already paused.")),
                State::Stopped => Err(anyhow!("Environment is stopped and cannot be paused.")),
            },
            None => Err(anyhow!("Environment does not exist.")),
        }
    }

    pub fn stop_environment<S: Into<String> + Clone>(
        &mut self,
        environment_label: S,
    ) -> Result<()> {
        match self.environments.get_mut(&environment_label.clone().into()) {
            Some(environment) => match environment.state.load(std::sync::atomic::Ordering::Relaxed)
            {
                State::Initialization => Err(anyhow!("Environment is not running.")),
                State::Running => {
                    let (handle, state) = self
                        .handles_and_states
                        .remove(&environment_label.into())
                        .unwrap();
                    state.store(State::Stopped, std::sync::atomic::Ordering::Relaxed);
                    handle.join().unwrap();
                    Ok(())
                }
                State::Paused => {
                    // TODO: GIVE THE RESTART LOGIC HERE TOO
                    let (handle, state) = self
                        .handles_and_states
                        .remove(&environment_label.into())
                        .unwrap();
                    state.store(State::Stopped, std::sync::atomic::Ordering::Relaxed);
                    handle.join().unwrap();
                    Ok(())
                }
                State::Stopped => Err(anyhow!("Environment is already stopped.")),
            },
            None => Err(anyhow!("Environment does not exist.")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use super::*;

    #[test]
    fn new_manager() {
        let manager = Manager::new();
        assert!(manager.environments.is_empty());
    }
}
