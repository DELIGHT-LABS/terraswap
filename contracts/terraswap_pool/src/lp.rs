use std::ops::{Div, Mul};

use crate::{
    msg::MSG_REPLY_ID_NEW_TOKEN_POOL,
    state::{load_asset_info, load_lp_addr, store_lp, STATE, TMP_NEW_TOKEN_POOL},
    ContractError,
};
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, BankMsg, CosmosMsg, Deps, DepsMut, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use terra_cosmwasm::TerraMsgWrapper;
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;
use terraswap::{
    asset::{Asset, AssetInfo},
    querier::query_supply,
};

pub fn new_pool_token(
    deps: DepsMut,
    pool_address: Addr,
    sender: Addr,
    asset_info: AssetInfo,
    symbol: String,
) -> Result<SubMsg<TerraMsgWrapper>, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    if state.owner != sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Ok(lp_address) = load_lp_addr(deps.as_ref(), asset_info.clone()) {
        return Err(ContractError::AlreadyExsit(lp_address.to_string()));
    }

    TMP_NEW_TOKEN_POOL.save(deps.storage, &asset_info).unwrap();

    Ok(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: None,
            code_id: state.token_code_id,
            msg: to_binary(&TokenInstantiateMsg {
                name: "terraswap pool liqudity token".to_string(),
                symbol: format!("lp{}", symbol),
                decimals: 6u8,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: pool_address.to_string(),
                    cap: None,
                }),
            })?,
            funds: vec![],
            label: "".to_string(),
        }),
        MSG_REPLY_ID_NEW_TOKEN_POOL,
    ))
}

pub fn reply_new_pool_token(deps: DepsMut, lp_address: Addr) -> StdResult<Attribute> {
    let asset_info = TMP_NEW_TOKEN_POOL.load(deps.storage).unwrap();
    store_lp(deps, asset_info, lp_address.clone()).unwrap();

    Ok(attr("lp_token", lp_address))
}

#[cfg(test)]
mod pool_token {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, MOCK_CONTRACT_ADDR};
    use terraswap::asset::AssetInfoRaw;

    use crate::state::{State, LPS_ASSET_INFO};

    static OWNER: &str = "owner0000";
    static TOKEN_CODE_ID: u64 = 1u64;
    static UNKONWN_OWNER: &str = "unkown0000";

    fn init(deps: DepsMut) {
        STATE
            .save(
                deps.storage,
                &State {
                    owner: Addr::unchecked(OWNER.to_string()),
                    token_code_id: TOKEN_CODE_ID,
                },
            )
            .unwrap();
    }

    #[test]
    fn normal_new_pool_token() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let res = new_pool_token(
            deps.as_mut(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Addr::unchecked(OWNER.to_string()),
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            "UST".to_string(),
        )
        .unwrap();

        assert_eq!(
            res,
            SubMsg::reply_on_success(
                CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id: TOKEN_CODE_ID,
                    msg: to_binary(&TokenInstantiateMsg {
                        name: "terraswap pool liqudity token".to_string(),
                        symbol: "lpUST".to_string(),
                        decimals: 6u8,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: MOCK_CONTRACT_ADDR.to_string(),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                    funds: vec![],
                    label: "".to_string()
                }),
                MSG_REPLY_ID_NEW_TOKEN_POOL,
            )
        )
    }

    #[test]
    fn fail_to_unkown_owner_new_pool_token() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        new_pool_token(
            deps.as_mut(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Addr::unchecked(UNKONWN_OWNER.to_string()),
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            "UST".to_string(),
        )
        .unwrap_err();
    }

    #[test]
    fn fail_to_duplicate_token_pool() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        LPS_ASSET_INFO
            .save(
                &mut deps.storage,
                AssetInfoRaw::NativeToken {
                    denom: "uusd".to_string(),
                }
                .as_bytes(),
                &Addr::unchecked(MOCK_CONTRACT_ADDR.to_string()),
            )
            .unwrap();

        new_pool_token(
            deps.as_mut(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Addr::unchecked(OWNER.to_string()),
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            "UST".to_string(),
        )
        .unwrap_err();
    }

    #[test]
    fn normal_reply_new_token_pool() {
        let mut deps = mock_dependencies(&[]);

        TMP_NEW_TOKEN_POOL
            .save(
                &mut deps.storage,
                &AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            )
            .unwrap();

        let res = reply_new_pool_token(
            deps.as_mut(),
            Addr::unchecked(MOCK_CONTRACT_ADDR.to_string()),
        )
        .unwrap();
        assert_eq!(res, attr("lp_token", MOCK_CONTRACT_ADDR));
    }

    #[test]
    #[should_panic]
    fn fail_to_duplicate_reply_new_pool_token() {
        let mut deps = mock_dependencies(&[]);
        LPS_ASSET_INFO
            .save(
                &mut deps.storage,
                AssetInfoRaw::NativeToken {
                    denom: "uusd".to_string(),
                }
                .as_bytes(),
                &Addr::unchecked(MOCK_CONTRACT_ADDR.to_string()),
            )
            .unwrap();
        reply_new_pool_token(
            deps.as_mut(),
            Addr::unchecked(MOCK_CONTRACT_ADDR.to_string()),
        )
        .unwrap();
    }
}

pub fn mint_pool_token(
    deps: Deps,
    pool_address: Addr,
    sender: Addr,
    asset: Asset,
) -> Result<CosmosMsg<TerraMsgWrapper>, ContractError> {
    let lp_token = load_lp_addr(deps, asset.info.clone()).unwrap();

    let total_asset = asset
        .info
        .query_pool(&deps.querier, deps.api, pool_address)
        .unwrap();

    let total_lp = query_supply(&deps.querier, lp_token.clone()).unwrap();

    let mint_amount: Uint128;
    if total_lp.is_zero() {
        mint_amount = asset.amount;
    } else {
        let prev_asset_amount = total_asset - asset.amount;
        let rate = prev_asset_amount.div(total_lp);
        mint_amount = prev_asset_amount / rate;
    }

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token.to_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            recipient: sender.to_string(),
            amount: mint_amount,
        })?,
    }))
}

#[cfg(test)]
mod mint_pool_token {
    use super::*;
    use crate::state::LPS_ASSET_INFO;
    use cosmwasm_std::{testing::MOCK_CONTRACT_ADDR, Coin};
    use terraswap::asset::AssetInfoRaw;
    use terraswap::mock_querier::mock_dependencies;

    static SENDER: &str = "sender0000";
    static LP_TOKEN: &str = "lptoken0000";

    fn init(deps: DepsMut) {
        let asset_info = AssetInfoRaw::NativeToken {
            denom: "uusd".to_string(),
        };
        LPS_ASSET_INFO
            .save(
                deps.storage,
                asset_info.as_bytes(),
                &Addr::unchecked(LP_TOKEN),
            )
            .unwrap();
    }

    #[test]
    fn normal_first_mint_token() {
        let mut deps = mock_dependencies(&[Coin {
            amount: Uint128::from(1u64),
            denom: "uusd".to_string(),
        }]);
        init(deps.as_mut());

        deps.querier.with_balance(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[Coin {
                amount: Uint128::from(1u64),
                denom: "uusd".to_string(),
            }],
        )]);
        deps.querier.with_token_balances(&[(
            &LP_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(0u128))],
        )]);

        let res = mint_pool_token(
            deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Addr::unchecked(SENDER),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(1u128),
            },
        )
        .unwrap();

        assert_eq!(
            res,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: SENDER.to_string(),
                    amount: Uint128::from(1u128),
                })
                .unwrap(),
            })
        )
    }

    #[test]
    fn normal_second_mint_token() {
        let mut deps = mock_dependencies(&[Coin {
            amount: Uint128::from(1u64),
            denom: "uusd".to_string(),
        }]);
        init(deps.as_mut());

        deps.querier.with_balance(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[Coin {
                amount: Uint128::from(2u64),
                denom: "uusd".to_string(),
            }],
        )]);
        deps.querier.with_token_balances(&[(
            &LP_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1u128))],
        )]);

        let res = mint_pool_token(
            deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Addr::unchecked(SENDER),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(1u128),
            },
        )
        .unwrap();

        assert_eq!(
            res,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: SENDER.to_string(),
                    amount: Uint128::from(1u128),
                })
                .unwrap(),
            })
        )
    }
}

pub fn burn_pool_token(
    deps: Deps,
    pool_address: Addr,
    sender: Addr,
    lp_token: Addr,
    amount: Uint128,
) -> Result<Vec<CosmosMsg<TerraMsgWrapper>>, ContractError> {
    let asset_info = load_asset_info(deps, lp_token.clone()).unwrap();

    let total_asset = asset_info
        .query_pool(&deps.querier, deps.api, pool_address)
        .unwrap();

    let total_lp = query_supply(&deps.querier, lp_token.clone()).unwrap();
    let rate = total_asset.div(total_lp);

    let unbond_amount = amount.mul(rate);

    let unbond_asset = Asset {
        amount: unbond_amount,
        info: asset_info.clone(),
    };

    let send_msg: CosmosMsg<TerraMsgWrapper> = match asset_info {
        AssetInfo::Token { contract_addr } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: unbond_amount,
            })
            .unwrap(),
            funds: vec![],
        }),
        AssetInfo::NativeToken { .. } => CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![unbond_asset.deduct_tax(&deps.querier).unwrap()],
        }),
    };

    Ok(vec![
        send_msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: lp_token.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
        }),
    ])
}

#[cfg(test)]
mod burn_pool_token {
    use cosmwasm_std::{coin, testing::MOCK_CONTRACT_ADDR, Coin};
    use terraswap::mock_querier::mock_dependencies;

    use super::*;
    use crate::state::LPS_LP_ADDR;

    static SENDER: &str = "sender0000";
    static LP_TOKEN: &str = "lptoken0000";

    fn init(deps: DepsMut) {
        let asset_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };
        LPS_LP_ADDR
            .save(deps.storage, Addr::unchecked(LP_TOKEN), &asset_info)
            .unwrap();
    }

    #[test]
    fn normal_burn_token() {
        let mut deps = mock_dependencies(&[Coin {
            amount: Uint128::from(1u64),
            denom: "uusd".to_string(),
        }]);
        init(deps.as_mut());

        deps.querier.with_balance(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[Coin {
                amount: Uint128::from(4u64),
                denom: "uusd".to_string(),
            }],
        )]);
        deps.querier.with_token_balances(&[(
            &LP_TOKEN.to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(2u128))],
        )]);

        let res = burn_pool_token(
            deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Addr::unchecked(SENDER),
            Addr::unchecked(LP_TOKEN),
            Uint128::from(1u64),
        )
        .unwrap();

        assert_eq!(
            res,
            vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: SENDER.to_string(),
                    amount: vec![coin(2u128, "uusd")],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: LP_TOKEN.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Burn {
                        amount: Uint128::from(1u128)
                    })
                    .unwrap(),
                }),
            ]
        )
    }
}
