use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Deps, DepsMut, StdError, StdResult};
use cw_storage_plus::{Item, Map};
use terraswap::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub token_code_id: u64,
}

pub const STATE: Item<State> = Item::new("state");

pub const LPS_ASSET_INFO: Map<&[u8], Addr> = Map::new("lps_asset_info");
pub const LPS_LP_ADDR: Map<Addr, AssetInfo> = Map::new("lps_lp_addr");

pub fn store_lp(deps: DepsMut, asset_info: AssetInfo, lp_address: Addr) -> StdResult<()> {
    if LPS_LP_ADDR.has(deps.storage, lp_address.clone()) {
        return Err(StdError::generic_err("already exist lp_token"));
    }
    LPS_LP_ADDR
        .save(deps.storage, lp_address.clone(), &asset_info)
        .unwrap();

    let asset_info_raw = asset_info.to_raw(deps.api).unwrap();
    if LPS_ASSET_INFO.has(deps.storage, asset_info_raw.as_bytes()) {
        return Err(StdError::generic_err("already exist asset_info"));
    }

    LPS_ASSET_INFO.save(deps.storage, asset_info_raw.as_bytes(), &lp_address)
}

pub fn load_lp_addr(deps: Deps, asset_info: AssetInfo) -> StdResult<Addr> {
    LPS_ASSET_INFO.load(
        deps.storage,
        asset_info.to_raw(deps.api).unwrap().as_bytes(),
    )
}

pub fn load_asset_info(deps: Deps, lp_address: Addr) -> StdResult<AssetInfo> {
    LPS_LP_ADDR.load(deps.storage, lp_address)
}

pub const TMP_NEW_TOKEN_POOL: Item<AssetInfo> = Item::new("tmp_new_token_pool");
