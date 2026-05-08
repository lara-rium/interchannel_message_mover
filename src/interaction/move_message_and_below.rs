use anyhow::Result;
use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::CommandBuilder;

use crate::interaction::InteractionContext;

pub const NAME: &str = "move this message and below";

pub fn command() -> Command {
    CommandBuilder::new(NAME, "", CommandType::Message)
        .dm_permission(false)
        .build()
}

impl InteractionContext<'_> {
    pub async fn handle_move_message_and_below_command(self) -> Result<()> {
        self.show_move_message_and_below_modal().await
    }
}
