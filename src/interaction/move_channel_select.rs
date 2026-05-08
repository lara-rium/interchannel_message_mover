use anyhow::Result;
use sparkle_convenience::{
    error::IntoError,
    interaction::{extract::InteractionDataExt, DeferBehavior, DeferVisibility},
    reply::Reply,
};
use twilight_model::{
    application::interaction::Interaction,
    channel::{Channel, ChannelType},
    guild::{PartialMember, Permissions},
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
};
use twilight_util::permission_calculator::PermissionCalculator;

use crate::{
    interaction::{channel_select_menu::ChannelSelectMenu, InteractionContext},
    Context, CustomError,
};

pub const CUSTOM_ID: &str = "move_channel";

impl<'a> InteractionContext<'a> {
    pub async fn wait_for_channel_select_interaction(&self) -> Result<Channel> {
        self.handle
            .defer_with_behavior(DeferVisibility::Ephemeral, DeferBehavior::Update)
            .await?;
        let channel_select_message = self
            .followup_with_channel_select_menu(
                "where do you want to move the message?".to_owned(),
                DeferVisibility::Ephemeral,
                ChannelSelectMenu::new(
                    CUSTOM_ID.to_owned(),
                    vec![
                        ChannelType::GuildText,
                        ChannelType::GuildAnnouncement,
                        ChannelType::AnnouncementThread,
                        ChannelType::PublicThread,
                        ChannelType::PrivateThread,
                    ],
                ),
            )
            .await?
            .model()
            .await?;

        let interaction = self
            .ctx
            .standby
            .wait_for_component(channel_select_message.id, |_: &Interaction| true)
            .await?;

        self.handle
            .reply(
                Reply::new()
                    .ephemeral()
                    .update_last()
                    .content("noted, doing some checks :face_with_monocle:"),
            )
            .await?;

        self.ctx.move_channel(interaction).await
    }
}

impl Context {
    pub async fn validate_destination_channel(
        &self,
        guild_id: Id<GuildMarker>,
        member: &PartialMember,
        channel_input: &str,
    ) -> Result<Channel> {
        let channel = if let Ok(channel_id) = channel_input.parse::<Id<ChannelMarker>>() {
            self.bot.http.channel(channel_id).await?.model().await?
        } else {
            let channels = self
                .bot
                .http
                .guild_channels(guild_id)
                .await?
                .models()
                .await?;
            channels
                .into_iter()
                .find(|ch| {
                    ch.name
                        .as_deref()
                        .map(|n| n.eq_ignore_ascii_case(channel_input))
                        .unwrap_or(false)
                })
                .ok_or_else(|| CustomError::ChannelNotFound(channel_input.to_owned()))?
        };

        let permission_overwrites = if channel.kind.is_thread() {
            self.bot
                .http
                .channel(channel.parent_id.ok()?)
                .await?
                .model()
                .await?
                .permission_overwrites
                .ok()?
        } else {
            channel.permission_overwrites.clone().ok()?
        };

        let guild = self.bot.http.guild(guild_id).await?.model().await?;

        let everyone_role = guild
            .roles
            .iter()
            .find_map(|role| (role.id.cast() == guild.id).then_some(role.permissions))
            .ok()?;

        let member_roles = guild
            .roles
            .iter()
            .filter_map(|role| {
                member
                    .roles
                    .contains(&role.id)
                    .then_some((role.id, role.permissions))
            })
            .collect::<Vec<_>>();

        let permissions = PermissionCalculator::new(
            guild.id,
            member.user.as_ref().ok()?.id,
            everyone_role,
            &member_roles,
        )
        .in_channel(channel.kind, &permission_overwrites);

        let required_permissions = if channel.kind.is_thread() {
            Permissions::SEND_MESSAGES_IN_THREADS
        } else {
            Permissions::SEND_MESSAGES
        };
        if !permissions.contains(required_permissions) {
            return Err(CustomError::SendMessagesPermissionMissing.into());
        }

        Ok(channel)
    }

    async fn move_channel(&self, interaction: Interaction) -> Result<Channel> {
        let channel_id = interaction
            .data
            .clone()
            .ok()?
            .component()
            .ok()?
            .values
            .into_iter()
            .next()
            .ok()?
            .parse::<Id<ChannelMarker>>()?;
        let member = interaction.member.as_ref().ok()?;

        let guild = self
            .bot
            .http
            .guild(interaction.guild_id.ok()?)
            .await?
            .model()
            .await?;
        let channel = self.bot.http.channel(channel_id).await?.model().await?;
        let permission_overwrites = if channel.kind.is_thread() {
            self.bot
                .http
                .channel(channel.parent_id.ok()?)
                .await?
                .model()
                .await?
                .permission_overwrites
                .ok()?
        } else {
            channel.permission_overwrites.clone().ok()?
        };

        let everyone_role = guild
            .roles
            .iter()
            .find_map(|role| (role.id.cast() == guild.id).then_some(role.permissions))
            .ok()?;

        let member_roles = guild
            .roles
            .iter()
            .filter_map(|role| {
                member
                    .roles
                    .contains(&role.id)
                    .then_some((role.id, role.permissions))
            })
            .collect::<Vec<_>>();

        let permissions = PermissionCalculator::new(
            guild.id,
            member.user.as_ref().ok()?.id,
            everyone_role,
            &member_roles,
        )
        .in_channel(channel.kind, &permission_overwrites);

        let required_permissions = if channel.kind.is_thread() {
            Permissions::SEND_MESSAGES_IN_THREADS
        } else {
            Permissions::SEND_MESSAGES
        };
        if !permissions.contains(required_permissions) {
            return Err(CustomError::SendMessagesPermissionMissing.into());
        }

        Ok(channel)
    }
}
