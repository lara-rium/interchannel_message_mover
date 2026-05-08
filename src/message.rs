use anyhow::Result;
use sparkle_convenience::error::IntoError;
use twilight_model::{
    channel::{Channel, Message},
    http::attachment::Attachment,
};

use crate::{Context, CustomError};

impl Context {
    pub async fn execute_webhook_as_member(
        &self,
        message: &Message,
        channel: &Channel,
    ) -> Result<bool> {
        let mut channel_id = channel.id;
        let mut thread_id = None;
        if channel.kind.is_thread() {
            thread_id = Some(channel_id);
            channel_id = channel.parent_id.ok()?;
        };

        let webhook = match self
            .bot
            .http
            .channel_webhooks(channel_id)
            .await?
            .models()
            .await?
            .into_iter()
            .find(|webhook| webhook.token.is_some())
        {
            Some(webhook) => webhook,
            None => {
                self.bot
                    .http
                    .create_webhook(channel_id, "interchannel message mover")?
                    .await?
                    .model()
                    .await?
            }
        };
        let webhook_token = webhook.token.ok()?;

        // Skip messages with nothing transferable (stickers, system messages, embed-only)
        // Return false to signal the caller not to delete this message from source
        if message.content.is_empty() && message.attachments.is_empty() {
            return Ok(false);
        }

        // Download each attachment from Discord CDN so we can re-upload them
        let mut attachment_files: Vec<Attachment> = Vec::new();
        for (idx, attachment) in message.attachments.iter().enumerate() {
            let bytes = reqwest::get(&attachment.url)
                .await?
                .bytes()
                .await?
                .to_vec();
            attachment_files.push(Attachment::from_bytes(
                attachment.filename.clone(),
                bytes,
                idx as u64,
            ));
        }

        let mut execute_webhook = self
            .bot
            .http
            .execute_webhook(webhook.id, &webhook_token)
            .username(
                message
                    .member
                    .as_ref()
                    .and_then(|member| member.nick.as_ref())
                    .unwrap_or(&message.author.name),
            )?;

        // Only set content if non-empty — Twilight rejects empty strings
        if !message.content.is_empty() {
            execute_webhook = execute_webhook
                .content(&message.content)
                .map_err(|_| CustomError::MessageTooLong)?;
        }

        if !attachment_files.is_empty() {
            execute_webhook = execute_webhook
                .attachments(&attachment_files)
                .map_err(|_| CustomError::MessageTooLong)?;
        }

        if let Some(thread_id) = thread_id {
            execute_webhook = execute_webhook.thread_id(thread_id);
        }

        if let Some(avatar_url) = message
            .member
            .as_ref()
            .and_then(|member| member.avatar)
            .zip(message.guild_id)
            .map(|(avatar, guild_id)| {
                format!(
                    "https://cdn.discordapp.com/guilds/{guild_id}/users/{}/avatar/{}.png",
                    message.author.id, avatar
                )
            })
            .or_else(|| {
                message.author.avatar.map(|avatar| {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png",
                        message.author.id, avatar
                    )
                })
            })
        {
            execute_webhook.avatar_url(&avatar_url).await?;
        } else {
            execute_webhook.await?;
        }

        Ok(true)
    }
}

pub fn check(_message: &Message) -> Result<()> {
    Ok(())
}
