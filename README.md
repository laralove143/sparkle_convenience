# ✨😌 Sparkle Convenience

- 🗄️ [GitHub](https://github.com/laralove143/sparkle-convenience)
- 📦 [crates.io](https://crates.io/crates/sparkle-convenience)
- 📖 [docs.rs](https://docs.rs/sparkle-convenience/latest)

A wrapper over [Twilight](https://github.com/twilight-rs/twilight) that's designed to be convenient to use, without
relying on callbacks and mostly following Twilight patterns while making your life easier

## ✨ Features

- Get your bot started with one method
- Defer, respond to or update responses of interactions without having to track anything yourself
- Extract interaction data easily
- Send timed messages that are deleted after a timeout
- Handle user errors with little boilerplate and with edge cases caught
- Log internal errors with webhooks
- Much more you can find out in the docs!

## 😋 A Taste of Convenience

There's [Sparkle Template](https://github.com/laralove143/sparkle-template), providing the boilerplate for Sparkle Convenience, this can also act as a full example usage

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

# 🚚 Cargo Features

- `anyhow`: Pulls the `anyhow` crate to provide convenience features around it in the `error` module

## ✉️ Contact

Feature Requests? Bugs? Support? Contributions? You name it, I'm always looking for community feedback from anyone who
uses my work!

If you have a question, [join Twilight's Discord server please](https://discord.gg/twilight-rs)
