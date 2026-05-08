use anyhow::Result;
use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::CommandBuilder;

use crate::interaction::InteractionContext;

pub const NAME: &str = "move message";

pub fn command() -> Command {
    CommandBuilder::new(NAME, "", CommandType::Message)
        .dm_permission(false)
        .build()
}

impl InteractionContext<'_> {
    pub async fn handle_move_message_command(self) -> Result<()> {
        self.show_move_message_modal().await
    }
}
