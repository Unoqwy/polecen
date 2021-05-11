# polecen

Polecen is a command arguments parser for [serenity][].  

## Current state

Polecen is currently in very early stages. Almost everything is subject to a refactor and/or breaking changes relatively soon.  

### "Working state" roadmap

- [x] Basic expansion
- [x] Default parsers implementations
- [ ] Integration with Standard Framework
- [ ] Usable errors
- [ ] FromStr support?
- [ ] Basic documentation

### Planned features

The following features are planned but no one knows when they will be implemented.  

* Interactions support
* CLI to manage interactions automatically according to expansion macros

## Macros example

You can declare a command using the powerful expand macro:

```rust
use polecen::arguments::prelude::*;
use serenity::model::guild::Member;

polecen::expand_command_here!((TestCommandArgs) test => match {
    kick => {
        target#Member "Target member";
        reason#String [*] "Reason"; // optional argument
    },
    version | ver | "?" => {}
});
```

Argument types must implement `ArgumentType`.  
Note: The feature `default_parsers` provides default implementations of ArgumentType for many std types and serenity models.  
For these parsers to be in scope, you must either use `polecen::arguments::prelude[::*]` or `polecen::arguments::default`.

Once a command has been declared, you can read the arguments using `read_args`:

```rust
// args is a &str iterable
// ctx is a serenity context
let args = polecen::read_args!(TestCommandArgs, args, ctx, [M] message)?; // ➾ TestCommandArgs
```

And later get values from the args' fields:

```rust
match &args {
    TestCommandArgs::Kick(args) => {
        // target is of type Member
        // reason is of type String
        let TestCommandArgsKick { target, reason } = args;

        /* do something with target and reason */
    },
    TestCommandArgs::Version => {
        /* do something */
    },
}
```

Please check the [examples](./examples) directory for more examples.

### Generated code

The example above would generate 2 structures:

```rust
#[derive(Clone, Debug)]
pub enum TestCommandArgs {
    Kick(TestCommandArgsKick),
    Version,
}

impl serenity::prelude::CommandArguments for TestCommandArgs {
    /* implementation of read_arguments */
}

#[derive(Clone, Debug)]
pub struct TestCommandArgsKick {
    pub target: Member,
    pub reason: Option<String>,
}
```

Upcoming improvements will include implementing CommandArguments for all structures.

[serenity]: https://github.com/serenity-rs/serenity
