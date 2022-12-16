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

## Example

There's a ping-pong command example in the docs for `Bot`, showcasing most of this crate's functionality

## Features

These are only the most commonly used features of the crate, other features can be found in the documentation

### Interactions

#### Responding

- Do all interaction handling without rewriting the ID and token using a handle
- Create a reply or followup response with a reply struct using the builder-pattern, which can be reused easily
- Defer an interaction with one method
- Create an autocomplete or modal response with minimal boilerplate

#### Extraction

- Extract the user of an interaction whether it's in DMs or not
- Extract the name of an interaction
- Extract the command data in an interaction with a method based on the interaction kind

### Errors

#### User Errors

- Easily check that the bot has the permissions required to run a command, and tell the user when it doesn't

#### Internal Errors

- Handle internal errors by printing them, writing them to a file and executing a webhook, all configurable
- All errors that should be ignored aren't emitted so that you don't flood your logs with errors
- Convert an option to an error with just `err.ok()`

### Initialization

- Create a `Bot` struct with just a token, intents and event types
- The `Bot` struct combines common Twilight data and provides abstraction methods on it

### HTTP

- DM a user with minimal boilerplate

## Looking for Ideas

The scope of this project includes anything in Twilight that could be more convenient to use, please make an
issue for anything that falls under this category!

## Caching

HTTP-fallback is not a good idea for many reasons, and there isn't much this crate could provide besides that, but
caching everything possible will give you a peace of mind
