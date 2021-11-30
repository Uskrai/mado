use super::{MadoMsg, Sender};

/// Mado state that can be shared and used to control [`MadoEngine`].
pub struct MadoEngineState {
    sender: super::Sender,
}

impl MadoEngineState {
    pub fn new(sender: Sender) -> Self {
        Self { sender }
    }

    pub fn send(&self, msg: MadoMsg) -> Result<(), tokio::sync::mpsc::error::SendError<MadoMsg>> {
        self.sender.send(msg)
    }
}
