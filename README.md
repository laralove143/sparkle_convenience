[TWILIGHT_REPO_LINK]: https://github.com/twilight-rs/twilight

[TWILIGHT_DISCORD_LINK]: https://discord.gg/twilight-rs

# ❓ RC INFO

This version isn't unstable, but it includes breaking changes. This isn't published at the next minor
version to follow Twilight's version.

RCs follow different versioning, meaning RC versions are breaking only between themselves. For example `0.1.0-rc.1`
isn't breaking with `0.1.1-rc.1`, but `0.1.0-rc.2` is breaking with it

# ✨😌 Sparkle Convenience

- 🗄️ [GitHub](https://github.com/laralove143/sparkle-convenience)
- 📦 [crates.io](https://crates.io/crates/sparkle-convenience)
- 📖 [docs.rs](https://docs.rs/sparkle-convenience/latest)

A wrapper over [Twilight][TWILIGHT_REPO_LINK] that's designed to be convenient to use, without
relying on callbacks and mostly following Twilight patterns while making your life easier

## ✨ FEATURES

- Get your bot started with one method
- Defer, respond to or update responses of interactions without having to track anything yourself
- Extract interaction data easily
- Send timed messages that are deleted after a timeout
- Handle user errors with little boilerplate and with edge cases caught
- Log internal errors with webhooks
- Much more you can find out in the docs!

## 😋 A TASTE OF CONVENIENCE

<!-- @formatter:off -->

```rust,ignore
let bot = Bot::new(
    "forgot to leak my token".to_owned(),
    Intents::empty(),
    EventTypeFlags::INTERACTION_CREATE,
)
.await?;

let handle = bot.interaction_handle(&interaction);
if interaction.name().ok()? == "pay_respects" {
    handle.defer(DeferVisibility::Ephemeral).await?;
    handle.check_permissions(Permissions::MANAGE_GUILD)?;
    let very_respected_user = interaction.data.ok()?.command().ok()?.target_id.ok()?;

    handle
        .reply(
            Reply::new()
                .ephemeral()
                .content("Paying respects".to_owned()),
        )
        .await?;

    handle
        .reply(
            Reply::new()
                .ephemeral()
                .update_last()
                .content(format!("<@{very_respected_user}> has +1 respect now")),
        )
        .await?;
}
```
<!-- @formatter:on -->

# 🚚 CARGO FEATURES

- `anyhow`: Pulls the `anyhow` crate to provide convenience features around it in the `error` module

## ✉️ CONTACT

Feature Requests? Bugs? Support? Contributions? You name it, I'm always looking for community feedback from anyone who
uses my work!

If you have a question, [join Twilight's Discord server please][TWILIGHT_DISCORD_LINK]
