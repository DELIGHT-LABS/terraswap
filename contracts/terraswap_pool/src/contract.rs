#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use protobuf::Message;
use terra_cosmwasm::TerraMsgWrapper;
use terraswap::asset::AssetInfo;

use crate::error::ContractError;
use crate::lp::{new_pool_token, reply_new_pool_token};
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, LpInfoResponse, QueryMsg,
    MSG_REPLY_ID_NEW_TOKEN_POOL,
};
use crate::response::MsgInstantiateContractResponse;
use crate::state::{load_lp_addr, State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:terraswap-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        token_code_id: msg.token_code_id,
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("token_code_id", msg.token_code_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg {
        ExecuteMsg::CreatePool { asset_info, symbol } => {
            try_create_pool(deps, env, info, asset_info, symbol)
        }
    }
}

pub fn try_create_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo,
    symbol: String,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let state = STATE.load(deps.storage).unwrap();
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let sub_msg =
        new_pool_token(deps, env.contract.address, info.sender, asset_info, symbol).unwrap();

    Ok(Response::new().add_submessage(sub_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetLpInfo { asset_info } => to_binary(&query_lp_info(deps, asset_info)),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(ConfigResponse {
        token_code_id: state.token_code_id,
        owner: state.owner,
    })
}

fn query_lp_info(deps: Deps, asset_info: AssetInfo) -> LpInfoResponse {
    let lp_address = load_lp_addr(deps, asset_info).unwrap();

    LpInfoResponse { lp_address }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg.id {
        MSG_REPLY_ID_NEW_TOKEN_POOL => {
            let data = msg.result.unwrap().data.unwrap();
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(data.as_slice())
                .map_err(|_| {
                    StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
                })?;
            let liquidity_token = res.get_contract_address();
            let attr = reply_new_pool_token(deps, Addr::unchecked(liquidity_token)).unwrap();
            Ok(Response::new().add_attribute(attr.key, attr.value))
        }
        _ => Err(ContractError::UnknownReplyId(msg.id)),
    }
}

#[cfg(test)]
mod instantiate {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { token_code_id: 1 };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(1, value.token_code_id);
        assert_eq!(Addr::unchecked("creator"), value.owner);
    }
}

#[cfg(test)]
mod create_pool {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{CosmosMsg, SubMsg, WasmMsg};
    use cw20::MinterResponse;
    use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

    static OWNER: &str = "owner0000";
    static TOKEN_CODE_ID: u64 = 1;

    fn init(deps: DepsMut) {
        let msg = InstantiateMsg {
            token_code_id: TOKEN_CODE_ID,
        };
        let info = mock_info(OWNER, &[]);

        // we can just call .unwrap() to assert this was a success
        instantiate(deps, mock_env(), info, msg).unwrap();
    }
    #[test]
    fn normal_create_pool() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let info = mock_info(OWNER, &[]);
        let msg = ExecuteMsg::CreatePool {
            asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            symbol: "UST".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new().add_submessage(SubMsg::reply_on_success(
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
                    label: "".to_string(),
                }),
                MSG_REPLY_ID_NEW_TOKEN_POOL,
            ))
        );
    }

    #[test]
    fn unknown_owner_create_pool_will_err() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let info = mock_info("unknown0000", &[]);
        let msg = ExecuteMsg::CreatePool {
            asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            symbol: "UST".to_string(),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    }
}

#[cfg(test)]
mod reply {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, ContractResult, SubMsgExecutionResponse};

    static OWNER: &str = "owner0000";
    static TOKEN_CODE_ID: u64 = 1;

    fn init(mut deps: DepsMut) {
        let msg = InstantiateMsg {
            token_code_id: TOKEN_CODE_ID,
        };
        let info = mock_info(OWNER, &[]);

        // we can just call .unwrap() to assert this was a success
        instantiate(deps.branch(), mock_env(), info, msg).unwrap();

        let info = mock_info(OWNER, &[]);
        let msg = ExecuteMsg::CreatePool {
            asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            symbol: "UST".to_string(),
        };
        execute(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn normal_reply_new_pool_token() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());
        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let res = reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: MSG_REPLY_ID_NEW_TOKEN_POOL,
                result: ContractResult::Ok(SubMsgExecutionResponse {
                    events: vec![],
                    data: Some(res.write_to_bytes().unwrap().into()),
                }),
            },
        )
        .unwrap();

        assert_eq!(res.attributes[0], attr("lp_token", MOCK_CONTRACT_ADDR));

        let res = query_lp_info(
            deps.as_ref(),
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        );

        assert_eq!(
            res,
            LpInfoResponse {
                lp_address: Addr::unchecked(MOCK_CONTRACT_ADDR)
            }
        )
    }

    #[test]
    fn fail_reply_new_pool_token_with_unknown_reply_response() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: MSG_REPLY_ID_NEW_TOKEN_POOL,
                result: ContractResult::Ok(SubMsgExecutionResponse {
                    events: vec![],
                    data: Some(to_binary(&QueryMsg::GetConfig {}).unwrap()),
                }),
            },
        )
        .unwrap_err();
    }

    #[test]
    fn unknown_reply_id_will_error() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: 99u64,
                result: ContractResult::Ok(SubMsgExecutionResponse {
                    events: vec![],
                    data: Some(res.write_to_bytes().unwrap().into()),
                }),
            },
        )
        .unwrap_err();
    }
}
