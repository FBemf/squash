#![warn(clippy::all)]

#[allow(dead_code)]
pub const TEXT: &str =
    "When you create a closure, Rust infers which \
     trait to use based on how the closure uses the values from the environment. All \
     closures implement FnOnce because they can all be called at least once. Closures \
     that don't move the captured variables also implement FnMut, and closures that \
     don't need mutable access to the captured variables also implement Fn. In Listing \
     13-12, the equal_to_x closure borrows x immutably (so equal_to_x has the Fn trait \
     ) because the body of the closure only needs to read the value in x.\n\
     If you want to force the closure to take ownership of the values it uses in the \
     environment, you can use the move keyword before the parameter list. This technique \
     is mostly useful when passing a closure to a new thread to move the data so it's \
     owned by the new thread.\n\
     We'll have more examples of move closures in Chapter 16 when we talk about concurrency. \
     For now, here's the code from Listing 13-12 with the move keyword added to the \
     closure definition and using vectors instead of integers, because integers can \
     be copied rather than moved; note that this code will not yet compile.";
