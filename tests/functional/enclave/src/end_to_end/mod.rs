// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::prelude::v1::*;
use teaclave_attestation::verifier;
use teaclave_config::RuntimeConfig;
use teaclave_config::BUILD_CONFIG;
use teaclave_proto::teaclave_authentication_service::*;
use teaclave_proto::teaclave_common::*;
use teaclave_proto::teaclave_frontend_service::*;
use teaclave_rpc::config::SgxTrustedTlsClientConfig;
use teaclave_rpc::endpoint::Endpoint;
use teaclave_types::*;

const USERNAME: &str = "alice";
const PASSWORD: &str = "daHosldOdker0sS";
const CONFIG_FILE: &str = "runtime.config.toml";
const AUTH_SERVICE_ADDR: &str = "localhost:7776";
const FRONTEND_SERVICE_ADDR: &str = "localhost:7777";

mod mesapy_echo;
mod native_echo;

pub fn run_tests() -> bool {
    use teaclave_test_utils::*;
    setup();
    run_tests!(
        native_echo::test_echo_task_success,
        mesapy_echo::test_echo_task_success,
    )
}

lazy_static! {
    static ref ENCLAVE_INFO: EnclaveInfo = {
        let runtime_config = RuntimeConfig::from_toml(CONFIG_FILE).expect("runtime config");
        EnclaveInfo::from_bytes(
            &runtime_config
                .audit
                .enclave_info_bytes
                .as_ref()
                .expect("encalve info"),
        )
    };
}

fn setup() {
    // Register user for the first time
    let mut api_client =
        create_authentication_api_client(&ENCLAVE_INFO, AUTH_SERVICE_ADDR).unwrap();
    register_new_account(&mut api_client, USERNAME, PASSWORD).unwrap();
}

fn create_client_config(
    enclave_info: &EnclaveInfo,
    service_name: &str,
) -> Result<SgxTrustedTlsClientConfig> {
    let enclave_attr = enclave_info
        .get_enclave_attr(service_name)
        .expect("enclave attr");
    let config = SgxTrustedTlsClientConfig::new().attestation_report_verifier(
        vec![enclave_attr],
        BUILD_CONFIG.as_root_ca_cert,
        verifier::universal_quote_verifier,
    );
    Ok(config)
}

fn create_frontend_client(
    enclave_info: &EnclaveInfo,
    service_addr: &str,
    cred: UserCredential,
) -> Result<TeaclaveFrontendClient> {
    let tls_config = create_client_config(&enclave_info, "teaclave_frontend_service")?;
    let channel = Endpoint::new(service_addr).config(tls_config).connect()?;

    let mut metadata = HashMap::new();
    metadata.insert("id".to_string(), cred.id);
    metadata.insert("token".to_string(), cred.token);

    let client = TeaclaveFrontendClient::new_with_metadata(channel, metadata)?;
    Ok(client)
}

fn create_authentication_api_client(
    enclave_info: &EnclaveInfo,
    service_addr: &str,
) -> Result<TeaclaveAuthenticationApiClient> {
    let tls_config = create_client_config(&enclave_info, "teaclave_authentication_service")?;
    let channel = Endpoint::new(service_addr).config(tls_config).connect()?;

    let client = TeaclaveAuthenticationApiClient::new(channel)?;
    Ok(client)
}

fn register_new_account(
    api_client: &mut TeaclaveAuthenticationApiClient,
    username: &str,
    password: &str,
) -> Result<()> {
    let request = UserRegisterRequest::new(username, password);
    let response = api_client.user_register(request)?;

    log::info!("User register: {:?}", response);

    Ok(())
}

fn login(
    api_client: &mut TeaclaveAuthenticationApiClient,
    username: &str,
    password: &str,
) -> Result<UserCredential> {
    let request = UserLoginRequest::new(username, password);
    let response = api_client.user_login(request)?;

    log::info!("User login: {:?}", response);

    Ok(UserCredential::new(username, response.token))
}