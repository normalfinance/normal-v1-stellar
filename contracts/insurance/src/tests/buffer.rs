extern crate std;

use normal::constants::{ONE_MILLION_QUOTE, THIRTEEN_DAY};
use pretty_assertions::assert_eq;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, Symbol, Vec,
};

use super::setup::{deploy_insurance_contract, deploy_token_contract};

use crate::{
    contract::{Insurance, InsuranceClient},
    msg::{ConfigResponse, StakedResponse},
    storage::{InsuranceFund, Stake},
    tests::setup::{ONE_DAY, ONE_WEEK},
};

