# Rack State Machine interaction with Machine, Switch

This document defines the combined state machines for **Machine** (each compute tray / managed host lifecycle), **Switch** (each switch), and **Rack** (collection of machines, switches, and power shelf). The diagram below shows all three and the transitions between the Rack state machine and the Machine/Switch state machines.

## Combined State Diagram (Machine, Switch, Rack)

```plantuml
@startuml
state "Machine (Each compute tray runs this)" as Machine {
    [*] --> M_Created: site-op inserts expected machine and site explorer explores
    M_Created --> M_DpuDiscovering : has DPUs
    M_Created --> M_HostInit : no DPUs
    M_DpuDiscovering --> M_DPUInit : all DPUs discovered
    M_DpuDiscovering --> M_HostInit : no DPUs / force_dpu_nic
    M_DPUInit --> M_HostInit : all DPUs ready
    M_HostInit --> M_Validation : validation needed
    M_HostInit --> M_Measuring : attestation
    M_HostInit --> M_BomValidating : BOM validation
    M_HostInit --> M_Ready : init complete
    M_Validation --> M_Ready : Done
    M_Measuring --> M_Ready : passed
    M_BomValidating --> M_Ready : valid
    M_Ready --> M_Assigned : instance requested
    M_Assigned --> M_WaitingForCleanup : instance deleted
    M_WaitingForCleanup --> M_Ready : cleanup Done
    M_Ready --> M_DPUReprovision : reprovision
    M_Ready --> M_HostReprovision : reprovision (non-NVL node only)
    M_DPUReprovision --> M_HostInit : Done
    M_HostReprovision --> M_Ready : Done
    M_Assigned --> M_Failed : Failure
    M_HostInit --> M_Failed : Failure
    M_Measuring --> M_Failed : Failure
    M_Ready --> M_ForceDeletion : Admin
    M_Assigned --> M_ForceDeletion : Admin
    M_Failed --> M_ForceDeletion : Admin
    M_ForceDeletion --> [*]
}

state "Switch (Each switch runs this)" as Switch {

    [*] --> S_Created : site-op inserts expected switch and site explorer explores
    S_Created --> S_Configuring : init complete
    state S_Configuring {
        [*] --> S_Configuring_RotateOsPassword
        state "RotateOsPassword" as S_Configuring_RotateOsPassword
    }
    S_Configuring_RotateOsPassword --> S_Validating : rotate password done
    state S_Validating {
        [*] --> S_Validating_ValidateComplete
        state "ValidateComplete" as S_Validating_ValidateComplete
    }
    S_Validating_ValidateComplete --> S_BomValidating : validation complete
    state S_BomValidating {
        [*] --> S_BomValidating_BomValidateComplete
        state "BomValidateComplete" as S_BomValidating_BomValidateComplete
    }
    S_BomValidating_BomValidateComplete --> S_Ready : BOM validation complete
    S_Ready --> S_Deleting : marked for deletion
    S_Ready --> S_ReProvisioning : reprovision requested
    S_ReProvisioning --> S_Ready : firmware upgrade Completed
    S_ReProvisioning --> Error : firmware upgrade Failed
    Error --> S_Deleting : marked for deletion
    S_Deleting --> [*] : final delete
    S_ReProvisioning   --> S_Ready : Done
    S_Initializing      --> S_Failed : Failure
    S_Configuring     --> S_Failed : Failure
    S_Validating    --> S_Failed : Failure
    S_BomValidating --> S_Failed : Failure
}

state "Rack (collection of machines switches power shelf)" as Rack {
    [*]                --> R_Created : site-op enters expected rack {rack-id and rack type} and site explore creates rack.
    R_Created          --> R_Initializing : machine or switch created with some rack ID\n(and expected rack type)
    R_Initializing     --> R_Discovering : any one machines, nvswitches discovered        
    R_Discovering      --> R_Maintenance : when all machines (M_Ready) and switches (S_Ready)\nrack sends S_ReProvisioning, M_HostReprovision\nIssue Provision to compute, switch to BKG
    R_Discovering        : Rack waits here till every node \n in this rack reaches ready
    state R_Maintenance {
        [*] --> R_Maintenance_RMS_Firmware_Updates
        state "RMS:Firmware Updates" as R_Maintenance_RMS_Firmware_Updates
        state "RMS:Configure NMX Cluster" as R_Maintenance_RMS_Configure_NMX_Cluster
        R_Maintenance_RMS_Firmware_Updates --> R_Maintenance_RMS_Configure_NMX_Cluster
        R_Maintenance        : Rack waits here till every node \n in this rack reaches ready
    }
    R_Maintenance      --> R_Validation : Validate Rack
    state R_Validation {
        [*] --> R_Validation_ValidateComplete
        state "ValidateComplete" as R_Validation_ValidateComplete
    }
    R_Validation_ValidateComplete --> R_Ready : On completion of validation and when all trays in rack = \n M_Ready && S_Ready
    R_Ready            --> R_Maintenance : Issue Rack-Reprovision \n to New Version
    R_Validation       --> R_Failure : Failure
    R_Maintenance      --> R_Failure : Timeout and Failures
}

' ========================================
' Transitions
'

R_Initializing --> S_Created : Check for newly created switches
R_Initializing --> M_Created : Check for newly created compute machines

R_Discovering --> S_Ready : Check for all switch ready
R_Discovering --> M_Ready : Check for all computes ready

R_Maintenance --> S_Ready : request switch Reprovision
R_Maintenance --> M_Ready : request compute Reprovision

R_Maintenance --> M_HostReprovision : request to exit switch Reprovision state
R_Maintenance --> S_ReProvisioning : request to exit compute Reprovision state

@enduml
```

---

## Switch State Machine Flow

The **Switch** state machine runs on each switch. The lifecycle runs from creation (site-op inserts expected switch, Site Explorer explores) through configuration (OS password rotation), validation, BOM validation, to Ready. From Ready a switch can be marked for deletion, or enter ReProvisioning (e.g. when the Rack requests firmware upgrade); reprovision can complete back to Ready or fail to Error and then Deleting.

### Switch High-Level Flow

<div style="width: 180%; background: white; margin-left: -40%;">
<!-- Keep the empty line after this or the diagram will break -->

```plantuml
@startuml
skinparam state {
  BackgroundColor White
}

state "Created" as S_Created
state "Configuring" as S_Configuring
state "Validating" as S_Validating
state "BomValidating" as S_BomValidating
state "Ready" as S_Ready
state "ReProvisioning" as S_ReProvisioning
state "Error" as S_Error
state "Deleting" as S_Deleting
state "Failed" as S_Failed

[*] --> S_Created : Site-op inserts expected switch\nSite Explorer explores

S_Created --> S_Configuring : init complete

S_Configuring --> S_Validating : rotate password done

S_Validating --> S_BomValidating : validation complete

S_BomValidating --> S_Ready : BOM validation complete

S_Ready --> S_Deleting : marked for deletion
S_Ready --> S_ReProvisioning : reprovision requested

S_ReProvisioning --> S_Ready : firmware upgrade completed\nor Done
S_ReProvisioning --> S_Error : firmware upgrade failed

S_Error --> S_Deleting : marked for deletion

S_Deleting --> [*] : final delete

S_Configuring --> S_Failed : Failure
S_Validating --> S_Failed : Failure
S_BomValidating --> S_Failed : Failure
@enduml
```
</div>

### Switch State Definitions

#### Created (S_Created)

- **Entry:** Site operator inserts the expected switch; Site Explorer explores and creates the switch entity.
- **Exit:** When initialization is complete, the switch moves to **Configuring**.

#### Configuring (S_Configuring)

- **Entry:** From Created when init is complete.
- **Exit:**  
  - To **Validating** when OS password rotation is done.  
  - To **Failed** on any failure.

**Substate:** RotateOsPassword — rotates the OS password as part of initial configuration.

#### Validating (S_Validating)

- **Entry:** From Configuring when rotate password is done.
- **Exit:**  
  - To **BomValidating** when validation is complete.  
  - To **Failed** on any failure.

**Substate:** ValidateComplete — represents completion of the validation step.

#### BomValidating (S_BomValidating)

- **Entry:** From Validating when validation is complete.
- **Exit:**  
  - To **Ready** when BOM validation is complete.  
  - To **Failed** on any failure.

**Substate:** BomValidateComplete — represents completion of BOM validation.

#### Ready (S_Ready)

- **Entry:** From BomValidating when BOM validation is complete.
- **Exit:**  
  - To **Deleting** when marked for deletion.  
  - To **ReProvisioning** when reprovision is requested (e.g. by Rack for firmware upgrade).

The switch is fully operational. The Rack state machine checks for all switches in S_Ready when moving from Discovering to Maintenance and from Maintenance to Validation/Ready.

#### ReProvisioning (S_ReProvisioning)

- **Entry:** From Ready when reprovision is requested (e.g. Rack issues S_ReProvisioning for firmware upgrade).
- **Exit:**  
  - To **Ready** when firmware upgrade completes (or Done).  
  - To **Error** when firmware upgrade fails.

#### Error

- **Entry:** From ReProvisioning when firmware upgrade fails.
- **Exit:** To **Deleting** when marked for deletion.

#### Deleting (S_Deleting)

- **Entry:** From Ready when marked for deletion, or from Error when marked for deletion.
- **Exit:** To terminal **[***]** when final delete completes.

#### Failed (S_Failed)

- **Entry:** From Configuring, Validating, or BomValidating on any failure.
- **Exit:** Handled by operator/admin (recovery or remediation; not shown in the main flow).

### Switch Interaction with Rack

The Rack state machine drives or observes the Switch state machine as follows:

| Rack state     | Effect on Switch |
|----------------|------------------|
| R_Discovering  | Rack checks that all switches are S_Ready before moving to R_Maintenance. |
| R_Maintenance  | Rack requests switch reprovision (drives S_ReProvisioning); tracks when switches return to S_Ready. Rack can request exit from S_ReProvisioning. |

These cross-state dependencies are shown in the combined diagram above.

---

## Machine Interaction with Rack

The Rack state machine drives or observes the Machine (compute) state machine as follows:

| Rack state     | Effect on Machine |
|----------------|-------------------|
| R_Initializing | Rack checks for newly created compute machines (M_Created) that belong to this rack. |
| R_Discovering  | Rack checks that all computes are M_Ready before moving to R_Maintenance. |
| R_Maintenance  | Rack requests compute reprovision (drives M_HostReprovision); tracks when computes return to M_Ready. Rack can request exit from M_HostReprovision. |

These cross-state dependencies are shown in the combined diagram above.

---

## Rack State Machine Flow

The **Rack** state machine represents a collection of machines (compute trays), switches, and power shelf. The rack lifecycle runs in coordination with the Machine and Switch state machines: the rack tracks when its child machines and switches are created and ready, drives maintenance (firmware updates and NMX cluster configuration), and reaches Ready when all trays in the rack are ready and validation is complete.

### Rack High-Level Flow

<div style="width: 180%; background: white; margin-left: -40%;">
<!-- Keep the empty line after this or the diagram will break -->

```plantuml
@startuml
skinparam state {
  BackgroundColor White
}

state "Created" as R_Created
state "Initializing" as R_Initializing
state "Discovering" as R_Discovering
state "Maintenance" as R_Maintenance
state "Validation" as R_Validation
state "Ready" as R_Ready
state "Failure" as R_Failure

[*] --> R_Created : Site-op enters expected rack\n(rack-id, rack type)\nSite Explorer creates rack

R_Created --> R_Initializing : Machine or switch created\nwith this rack ID and expected rack type

R_Initializing --> R_Discovering : At least one machine or\nNVSwitch discovered

R_Discovering --> R_Maintenance : All machines M_Ready\nand all switches S_Ready;\nissue reprovision to BKG

R_Maintenance --> R_Validation : Validate Rack

R_Validation --> R_Ready : Validation complete and\nall trays M_Ready && S_Ready

R_Validation --> R_Failure : Failure

R_Ready --> R_Maintenance : Rack-Reprovision to new version

R_Maintenance --> R_Failure : Timeout and failures

note right of R_Discovering
  Rack waits until every node
  in this rack reaches ready
end note

note right of R_Maintenance
  Rack waits until every node
  in this rack reaches ready
end note
@enduml
```
</div>

### Rack State Definitions

#### Created (R_Created)

- **Entry:** Site operator enters the expected rack (rack-id and rack type); Site Explorer creates the rack entity.
- **Exit:** When a machine or switch is created with this rack ID and the expected rack type, the rack moves to **Initializing**.

The rack exists in the system but has no discovered children yet.

#### Initializing (R_Initializing)

- **Entry:** From Created when at least one machine or switch is created with this rack ID and expected rack type.
- **Exit:** When at least one machine or NVSwitch is discovered, the rack moves to **Discovering**.

During this state the system checks for newly created switches (S_Created) and newly created compute machines (M_Created) that belong to this rack.

#### Discovering (R_Discovering)

- **Entry:** From Initializing when any machine or NVSwitch is discovered for this rack.
- **Exit:** When **all** machines in the rack are in M_Ready and **all** switches are in S_Ready, the rack triggers reprovision (S_ReProvisioning for switches, M_HostReprovision for computes), issues provision to compute and switch to BKG, and moves to **Maintenance**.

The rack remains in Discovering until every node in the rack reaches ready. The state machine checks for all switches ready (S_Ready) and all computes ready (M_Ready) to decide when to transition.

#### Maintenance (R_Maintenance)

- **Entry:** From Discovering when all machines and switches in the rack are ready and reprovision has been issued (transition to BKG).
- **Exit:**  
  - To **Validation** when "Validate Rack" is completed.  
  - To **Failure** on timeout or other failures.

**Substates:**

1. **RMS: Firmware Updates** — Rack-level firmware update phase.
2. **RMS: Configure NMX Cluster** — NMX cluster configuration; entered after firmware updates complete.

The rack waits in Maintenance until every node in the rack reaches ready again after reprovision. From this state the rack can request switch Reprovision (driving S_ReProvisioning) and compute Reprovision (M_HostReprovision), and tracks when switches and computes return to S_Ready and M_Ready.

- **Failure:** R_Maintenance → R_Failure on timeout or failures.

#### Validation (R_Validation)

- **Entry:** From Maintenance when "Validate Rack" is triggered.
- **Exit:**  
  - To **Ready** when validation is complete and all trays in the rack are M_Ready and S_Ready.  
  - To **Failure** on any validation failure.

**Substate:** ValidateComplete — represents completion of the validation step before the rack can transition to Ready.

#### Ready (R_Ready)

- **Entry:** From Validation when validation is complete and every tray in the rack is M_Ready and S_Ready.
- **Exit:** To **Maintenance** when a Rack-Reprovision to a new version is issued.

The rack is fully operational and can accept a new reprovision request to move back into Maintenance.

#### Failure (R_Failure)

- **Entry:**  
  - From Validation on failure.  
  - From Maintenance on timeout or failures.
- **Exit:** Handled by operator/admin (recovery or remediation; not shown in the main flow).

### Rack Interaction with Machine and Switch

The Rack state machine coordinates with the Machine and Switch state machines as follows:

| Rack state     | Direction / effect |
|----------------|--------------------|
| R_Initializing | Checks for newly created switches → S_Created; checks for newly created compute machines → M_Created. |
| R_Discovering  | Checks for all switches ready → S_Ready; checks for all computes ready → M_Ready. |
| R_Maintenance  | Requests switch Reprovision → S_ReProvisioning; requests compute Reprovision → M_HostReprovision. Tracks when switches and computes return to S_Ready and M_Ready. Rack can request exit from switch Reprovision (S_ReProvisioning) and from compute Reprovision (M_HostReprovision). |

These cross-state dependencies are shown in the combined diagram above.

---

## Expected Rack API Design

The following design mirrors the Expected Machine API. The rack state machine is driven when the site operator enters an expected rack (rack-id and rack type) and Site Explorer creates the rack. Expected Rack describes the set of Racks that are expected to be managed by the Carbide instance.

**RPCs (design)**

| RPC | Request | Response | Description |
|-----|---------|----------|-------------|
| AddExpectedRack | ExpectedRack | Empty | Add one expected rack. |
| DeleteExpectedRack | ExpectedRackRequest | Empty | Delete one expected rack (by rack id). |
| UpdateExpectedRack | ExpectedRack | Empty | Update expected rack (e.g. rack type, expected trays). |
| GetExpectedRack | ExpectedRackRequest | ExpectedRack | Get one expected rack by id. |
| GetAllExpectedRacks | Empty | ExpectedRackList | Get all expected racks in the site. |
| ReplaceAllExpectedRacks | ExpectedRackList | Empty | Replace the entire expected racks table. |
| DeleteAllExpectedRacks | Empty | Empty | Delete all expected racks in the site. |
| GetAllExpectedRacksLinked | Empty | LinkedExpectedRackList | Get expected racks with links to explored racks / machines / switches. |

**Main types (design)**

- **ExpectedRack** — `rack_id` (or `id`), `rack_type`, optional **`rack_topology`** (see below), `expected_compute_trays`, `expected_power_shelves`, `expected_nvlink_switches`, `metadata`, timestamps, etc. Used by the site operator to declare an expected rack before Site Explorer creates the Rack entity.
- **ExpectedRackRequest** — `rack_id` (or optional `id`) to select one expected rack.
- **ExpectedRackList** — `repeated ExpectedRack expected_racks`.
- **LinkedExpectedRack** — links expected rack to explored rack id, list of machine/switch/power shelf ids, and `expected_rack_id`.

**Rack topology**

Rack topology describes the expected physical or logical layout of the rack so that discovery and validation can match actual machines, switches, and power shelves to expected positions or counts.

- **RackTopology** (design) — Optional on ExpectedRack. May include:
  - **Slot layout** — Which slots (or positions) are expected to hold compute trays, NVLink switches, or power shelves (e.g. slot indices or ranges).
  - **Counts or IDs** — Expected counts per type (`expected_compute_tray_count`, `expected_switch_count`, `expected_power_shelf_count`) or explicit expected identifiers per slot.
  - **Topology template / reference** — Optional reference (e.g. `topology_id` or `rack_type`) to a predefined topology (e.g. DGX rack, HGX rack) that implies a standard layout.

Rack topology is used when transitioning from **R_Initializing** to **R_Discovering** (matching discovered machines and switches to this rack by rack ID and optionally by topology) and when validating that the rack is complete (all expected slots or counts satisfied).

**DB design**

The **racks** table is the parent; **machines** and **switches** reference it via `rack_id`.

- **racks** — Primary key `id` (VARCHAR(64), rack identifier). One row per rack.
- **machines.rack_id** — Foreign key → **racks(id)**. Links the machine (compute tray) to its rack.
- **switches.rack_id** — Foreign key → **racks(id)**. Links the switch to its rack.
- **switches.switch_reprovision_request** — Boolean, default `false`. When true, indicates a reprovision has been requested for this switch (e.g. by the Rack state machine driving S_ReProvisioning).

Example constraints (design):

```sql
ALTER TABLE machines
  ADD CONSTRAINT fk_machines_rack_id
  FOREIGN KEY (rack_id) REFERENCES racks(id) ON DELETE SET NULL;

ALTER TABLE switches
  ADD CONSTRAINT fk_switches_rack_id
  FOREIGN KEY (rack_id) REFERENCES racks(id) ON DELETE SET NULL;

ALTER TABLE switches
  ADD COLUMN IF NOT EXISTS switch_reprovision_request BOOLEAN NOT NULL DEFAULT false;
```

Indexes on `rack_id` in **machines** and **switches** support lookups by rack (e.g. “all machines/switches in this rack”).

This API supports the transition **R_Created**: site-op enters expected rack (rack-id and rack type) and Site Explorer creates the rack when matching machines or switches are discovered with that rack ID.
