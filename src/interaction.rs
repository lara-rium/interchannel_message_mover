use anyhow::Result;
use sparkle_convenience::{
    error::IntoError,
    interaction::{extract::InteractionExt, InteractionHandle},
    Bot,
};
use twilight_model::application::interaction::{Interaction, InteractionType};

use crate::{err_reply, Context, CustomError, Error};

mod channel_select_menu;
mod message_command;
mod modal;
mod move_channel_select;
mod move_message;
mod move_message_and_below;

struct InteractionContext<'ctx> {
    ctx: &'ctx Context,
    handle: InteractionHandle<'ctx>,
    interaction: Interaction,
}

impl<'ctx> InteractionContext<'ctx> {
    async fn handle(self) -> Result<()> {
        if self.interaction.kind == InteractionType::ModalSubmit {
            return self.handle_modal_submit().await;
        }
        match self.interaction.name().ok()? {
            move_message::NAME => self.handle_move_message_command().await,
            move_message_and_below::NAME => self.handle_move_message_and_below_command().await,
            move_channel_select::CUSTOM_ID => Ok(()),
            name => Err(Error::UnknownCommand(name.to_owned()).into()),
        }
    }
}

pub async fn set_commands(bot: &Bot) -> Result<()> {
    let commands = &[move_message::command(), move_message_and_below::command()];

    bot.interaction_client()
        .set_global_commands(commands)
        .await?;

    Ok(())
}

impl Context {
    pub async fn handle_interaction(&self, interaction: Interaction) {
        let handle = self.bot.interaction_handle(&interaction);
        let ctx = InteractionContext {
            ctx: self,
            handle: handle.clone(),
            interaction,
        };

        if let Err(err) = ctx.handle().await {
            handle
                .handle_error::<CustomError>(err_reply(&err), err)
                .await;
        }
    }
}
