#[macro_export]
macro_rules! read_args {
    ($ty:ty, $args:expr, $ctx:expr, $guild_id:expr) => {
        <$ty>::read_arguments(
            $args,
            ::polecen::arguments::parse::ArgumentParseContext::new($ctx, $guild_id),
        )
        .await
    };
    ($ty:ty, $args:expr, $ctx:expr, [M] $message:ident) => {
        ::polecen::read_args!($ty, $args, $ctx, $message.guild_id)
    };
}
