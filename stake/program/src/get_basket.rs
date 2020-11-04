use serum_pool::context::PoolContext;
use serum_pool_schema::{Basket, PoolState};
use serum_stake::error::{StakeError, StakeErrorCode};

pub fn handler(
    ctx: &PoolContext,
    state: &PoolState,
    spt_amount: u64,
    is_creation: bool,
) -> Result<Basket, StakeError> {
    // TODO: check the semantics of this make sense.
    //
    // If no pool tokens are in circulation, then to create `spt_amount`
    // one must deposit the same `spt_amount`. Otherwise, take a
    // `simple_basket`.
    if ctx.total_pool_tokens()? == 0 {
        let quantities = match state.assets.len() {
            1 => vec![spt_amount as i64],
            2 => vec![0i64, spt_amount as i64],
            _ => return Err(StakeErrorCode::InvalidState)?,
        };
        Ok(Basket { quantities })
    } else {
        ctx.get_simple_basket(spt_amount, is_creation)
            .map_err(Into::into)
    }
}
