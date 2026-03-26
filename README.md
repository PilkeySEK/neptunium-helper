# neptunium-helper

Helper bot for the fluxer-neptunium community on Fluxer, made using fluxer-neptunium, of course.

Currently, the bot only supports a message that when reacted to with the right emoji adds or removes a role from/to the user. The message ID, role ID, guild ID, and emoji ID can be configured in the config. Doesn't support having unicode emojis (aka standard fluxer emojis) as reaction role because I use a custom one in the fluxer-neptunium community (and this bot isn't meant to be an all-in-one-cover-all-use-cases reaction role bot).

# How to run it yourself (why?)

You can run this bot yourself by copying `config.example.json` to `example.json` and filling in the fields, then use `cargo run`.