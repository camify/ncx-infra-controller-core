/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use carbide_uuid::switch::SwitchId;
use db::DatabaseError;
use model::expected_switch::ExpectedSwitch;
use model::site_explorer::ExploredManagedSwitch;
use sqlx::{PgConnection, PgPool};

use crate::CarbideResult;
use crate::site_explorer::SiteExplorerConfig;
use crate::site_explorer::explored_endpoint_index::ExploredEndpointIndex;
use crate::site_explorer::metrics::SiteExplorationMetrics;

pub struct SwitchCreator {
    database_connection: PgPool,
    config: SiteExplorerConfig,
}

impl SwitchCreator {
    pub fn new(database_connection: PgPool, config: SiteExplorerConfig) -> Self {
        Self {
            database_connection,
            config,
        }
    }

    pub(crate) async fn create_switches(
        &self,
        metrics: &mut SiteExplorationMetrics,
        explored_managed_switches: &[ExploredManagedSwitch],
        expected_explored_endpoint_index: &ExploredEndpointIndex,
    ) -> CarbideResult<()> {
        for explored_managed_switch in explored_managed_switches {
            let expected_switch = match expected_explored_endpoint_index
                .matched_expected_switch(&explored_managed_switch.bmc_ip)
            {
                Some(expected_switch) => expected_switch,
                None => continue,
            };

            match self
                .create_managed_switch(
                    explored_managed_switch,
                    expected_switch,
                    &self.database_connection,
                )
                .await
            {
                Ok(true) => {
                    metrics.created_switches_count += 1;
                    if metrics.created_switches_count as u64 == self.config.switches_created_per_run
                    {
                        break;
                    }
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::error!(
                        %error,
                        "Failed to create managed switch {:#?}",
                        explored_managed_switch.bmc_ip
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn create_managed_switch(
        &self,
        explored_managed_switch: &ExploredManagedSwitch,
        expected_switch: &ExpectedSwitch,
        pool: &PgPool,
    ) -> CarbideResult<bool> {
        let mut txn = pool
            .begin()
            .await
            .map_err(|e| DatabaseError::new("begin create_managed_switch", e))?;

        let created = self
            .create_switch(&mut txn, explored_managed_switch, expected_switch)
            .await?
            .is_some();

        if !created {
            txn.commit()
                .await
                .map_err(|e| DatabaseError::new("commit create_managed_switch", e))?;
            return Ok(false);
        }

        txn.commit()
            .await
            .map_err(|e| DatabaseError::new("commit create_managed_switch", e))?;

        Ok(true)
    }

    // Returns SwitchId if switch was created.
    async fn create_switch(
        &self,
        txn: &mut PgConnection,
        explored_managed_switch: &ExploredManagedSwitch,
        expected_switch: &ExpectedSwitch,
    ) -> CarbideResult<Option<SwitchId>> {
        let nv_os_mac_addresses = if explored_managed_switch.nv_os_mac_addresses.is_empty() {
            match expected_switch.nvos_mac_address {
                // This is to accomidate bug in redfish BMC
                Some(nvos_mac_address) => vec![nvos_mac_address],
                None => vec![],
            }
        } else {
            explored_managed_switch.nv_os_mac_addresses.clone()
        };
        for mac_address in &nv_os_mac_addresses {
            if db::switch::find_by_host_mac_address(txn, mac_address)
                .await?
                .is_some()
            {
                // already exists, skip
                return Ok(None);
            }
        }
        let switch_id = explored_managed_switch
            .clone()
            .report
            .generate_switch_id()?
            .unwrap();

        tracing::info!(%switch_id, "switch ID generated");

        let existing_switch = db::switch::find_by_id(txn, &switch_id).await?;

        if let Some(_existing_switch) = existing_switch {
            //Possibly multiple eth ports are connected?
            tracing::warn!(
                %switch_id,
                "Switch already exists, skipping. Potentially multiple eth ports with same switch host serial number?"
            );
            return Ok(None);
        }

        self.create_switch_from_explored_switch(
            txn,
            explored_managed_switch,
            expected_switch,
            switch_id,
        )
        .await?;

        Ok(Some(switch_id))
    }

    async fn create_switch_from_explored_switch(
        &self,
        txn: &mut PgConnection,
        _explored_switch: &ExploredManagedSwitch,
        expected_switch: &ExpectedSwitch,
        switch_id: SwitchId,
    ) -> CarbideResult<()> {
        let name = match expected_switch.metadata.name.is_empty() {
            true => expected_switch.serial_number.to_string(),
            false => expected_switch.metadata.name.to_string(),
        };
        let config = model::switch::SwitchConfig {
            name,
            enable_nmxc: false,
            fabric_manager_config: None,
            location: Some("US/CA/DC/San Jose/1000 N Mathilda Ave".to_string()),
        };
        let new_switch = model::switch::NewSwitch {
            id: switch_id,
            config,
        };

        _ = db::switch::create(txn, &new_switch).await?;
        // let bmc_info = explored_switch.bmc_info();
        // let hardware_info = HardwareInfo::default(); //TODO: Add this later when we have hardware info
        // self.update_switch_topology(txn, &switch_id, bmc_info, hardware_info)
        //     .await?;
        Ok(())
    }

    // async fn update_switch_topology(
    //     &self,
    //     _txn: &mut PgConnection,
    //     _switch_id: &SwitchId,
    //     _bmc_info: BmcInfo,
    //     _hardware_info: HardwareInfo,
    // ) -> CarbideResult<()> {
    //     //TODO Add this later when and if required
    //     Ok(())
    // }
}
