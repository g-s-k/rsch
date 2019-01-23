use parsley::prelude::*;
use parsley::Error;


#[test]
fn sicp_2_1() -> Result<(), Error> {
    let mut ctx = Context::base();
    let ex_1 = include_str!("sicp/ch2/ex_1.ss");
    ctx.run(ex_1)?;

    assert_eq!(
        ctx.run("(add-rat one-half one-third)")?,
        ctx.run("(cons 5 6)")?
    );

    assert_eq!(
        ctx.run("(mul-rat one-half one-third)")?,
        ctx.run("(cons 1 6)")?
    );

    assert_eq!(
        ctx.run("(add-rat one-third one-third)")?,
        ctx.run("(cons 2 3)")?
    );

    Ok(())
}
