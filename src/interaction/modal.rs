use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use sparkle_convenience::{
    error::IntoError,
    interaction::{extract::InteractionDataExt, DeferVisibility},
    reply::Reply,
};
use twilight_model::{
    channel::message::component::{TextInput, TextInputStyle},
    id::{marker::ChannelMarker, Id},
};

use crate::{interaction::InteractionContext, message};

const CHANNEL_INPUT_ID: &str = "channel";

impl InteractionContext<'_> {
    pub async fn show_move_message_modal(self) -> Result<()> {
        let message = self.handle_message_command()?;
        self.handle
            .modal(
                format!("move:{}:{}", message.channel_id, message.id),
                "Move Message".to_owned(),
                channel_text_input(),
            )
            .await?;
        Ok(())
    }

    pub async fn show_move_message_and_below_modal(self) -> Result<()> {
        let message = self.handle_message_command()?;
        self.handle
            .modal(
                format!("move_below:{}:{}", message.channel_id, message.id),
                "Move Messages".to_owned(),
                channel_text_input(),
            )
            .await?;
        Ok(())
    }

    pub async fn handle_modal_submit(self) -> Result<()> {
        let guild_id = self.interaction.guild_id.ok()?;
        let member = self.interaction.member.clone().ok()?;

        let modal_data = self.interaction.data.clone().ok()?.modal().ok()?;
        let custom_id = modal_data.custom_id.clone();

        let (and_below, rest) = if let Some(r) = custom_id.strip_prefix("move_below:") {
            (true, r.to_owned())
        } else if let Some(r) = custom_id.strip_prefix("move:") {
            (false, r.to_owned())
        } else {
            return Err(anyhow::anyhow!("unknown modal custom_id: {custom_id}"));
        };

        let (source_channel_str, message_id_str) = rest.split_once(':').ok()?;
        let source_channel_id: Id<ChannelMarker> = source_channel_str.parse()?;
        let message_id = message_id_str.parse()?;

        let channel_input = modal_data
            .components
            .into_iter()
            .flat_map(|row| row.components)
            .find(|c| c.custom_id == CHANNEL_INPUT_ID)
            .ok()?
            .value
            .ok()?;
        let channel_input = channel_input.trim().trim_start_matches('#').to_owned();

        self.handle.defer(DeferVisibility::Ephemeral).await?;

        let channel = self
            .ctx
            .validate_destination_channel(guild_id, &member, &channel_input)
            .await?;

        let first_message = self
            .ctx
            .bot
            .http
            .message(source_channel_id, message_id)
            .await?
            .model()
            .await?;

        let mut messages = vec![first_message];
        if and_below {
            let mut channel_messages = self
                .ctx
                .bot
                .http
                .channel_messages(messages[0].channel_id)
                .after(messages[0].id)
                .await?
                .models()
                .await?;
            channel_messages.reverse();
            messages.append(&mut channel_messages);
        }

        for msg in &messages {
            message::check(msg)?;
        }

        let reply_content = if !and_below {
            "starting up the bike :motor_scooter:"
        } else {
            match messages.len() {
                0..=10 => "starting up the car :red_car:",
                11..=20 => "starting up the truck :pickup_truck:",
                21..=30 => "starting up the truck :truck:",
                31..=40 => "starting up the lorry :articulated_lorry:",
                _ => "starting up the ship :ship:",
            }
        };
        self.handle
            .reply(Reply::new().ephemeral().update_last().content(reply_content))
            .await?;

        let mut sent_messages = Vec::new();
        for (idx, msg) in messages.iter().enumerate() {
            if (idx + 1) % 10 == 0 {
                println!(
                    "moving messages in {guild_id}: {}/{}",
                    idx + 1,
                    messages.len()
                );
            }
            let sent = self.ctx.execute_webhook_as_member(msg, &channel).await?;
            if sent {
                sent_messages.push(msg);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if sent_messages.is_empty() {
            println!("{guild_id} nothing to delete");
        } else if (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
            - u64::try_from(messages[0].timestamp.as_secs())?)
            > 2 * 7 * 24 * 60 * 60
            || sent_messages.len() == 1
        {
            for (idx, msg) in sent_messages.iter().enumerate() {
                if (idx + 1) % 10 == 0 {
                    println!(
                        "deleting messages in {guild_id}: {}/{}",
                        idx + 1,
                        sent_messages.len()
                    );
                }
                self.ctx
                    .bot
                    .http
                    .delete_message(msg.channel_id, msg.id)
                    .await?;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        } else {
            self.ctx
                .bot
                .http
                .delete_messages(
                    messages[0].channel_id,
                    &sent_messages.iter().map(|m| m.id).collect::<Vec<_>>(),
                )?
                .await?;
        }

        self.handle
            .reply(
                Reply::new()
                    .ephemeral()
                    .update_last()
                    .content("done :incoming_envelope:"),
            )
            .await?;

        println!("{guild_id} done");

        Ok(())
    }
}

fn channel_text_input() -> Vec<TextInput> {
    vec![TextInput {
        custom_id: CHANNEL_INPUT_ID.to_owned(),
        label: "Destination channel (name or ID)".to_owned(),
        max_length: None,
        min_length: None,
        placeholder: Some("#general".to_owned()),
        required: Some(true),
        style: TextInputStyle::Short,
        value: None,
    }]
}
