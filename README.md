# Sparkle Convenience

[GitHub](https://github.com/laralove143/sparkle-convenience) 
[crates.io](https://crates.io/crates/sparkle-convenience)
[docs.rs](https://docs.rs/sparkle-convenience/latest)

A wrapper over [Twilight](https://github.com/twilight-rs/twilight) that's designed to be convenient to use, without
relying on callbacks and mostly following Twilight patterns while making your life easier

## You should use this if you:

- Simply want something easy to use
- Don't like writing boilerplate
- Don't like thinking too much about the structure of your code

## You shouldn't use this if you:

- Want the maximum performance you can get
- Want to customize as much as you can
- Want something more low-level
- Are willing to give up the pros of this crate for these

## Interaction Handling

Provides methods to handle interactions conveniently:

- Do all interaction handling without rewriting the ID and token using a handle
- Create a followup response with a reply struct using the builder-pattern, which can be reused easily
- Defer an interaction with one method
- Create an autocomplete or modal response with minimal boilerplate

## Error Handling

Provides an enum to conveniently handle errors:

- The error enum combines user and internal errors
- Easily check that the bot has the permissions required to run a command, and tell the user when it doesn't
- Handle internal errors by printing them, writing them to a file and executing a webhook, all optionally

## Looking for Ideas

The scope of this project includes anything in Twilight that could be more convenient to use, please make an
issue for anything that falls under this category!

## Caching

HTTP-fallback is not a good idea for many reasons, and there isn't much this crate could provide besides that, but
caching everything possible will give you a peace of mind. If memory usage is a concern, consider
using [Sparkle Cache](https://github.com/laralove143/sparkle-cache)
