window.console = new this.__bootstrap.console.Console((msg, level) =>
    Deno.core.print(msg, level > 1)
);
