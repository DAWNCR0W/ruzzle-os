# Ruzzle OS IPC Protocols (v0.1)

This document defines the binary IPC protocol used by Ruzzle OS user-space modules.
All messages are encoded as TLV sequences to reinforce the “puzzle piece” contract:
modules only rely on shared shapes (types), not internal implementations.

---

## 1. TLV Format (Common)

Each message is a sequence of **TLV fields**:

```
+---------+---------+-----------------+
| Type    | Length  | Value           |
| u16 LE  | u16 LE  | Length bytes    |
+---------+---------+-----------------+
```

Rules:
- Fields may appear in any order.
- Unknown TLVs must be ignored by receivers.
- Duplicate required fields are invalid.
- Strings are UTF-8 and must be non-empty.

---

## 2. Console Service Protocol (`ruzzle.console`)

Purpose: structured logging from modules to console-service.

### TLV Types
- `1` `TLV_LEVEL`  (u8)
- `2` `TLV_PID`    (u32 LE)
- `3` `TLV_MESSAGE` (UTF-8 string)

### Message
A log record contains exactly one of each field.

---

## 3. Init Registry Protocol (`ruzzle.init.registry`)

Purpose: modules register and lookup services through init.

### TLV Types
- `1` `TLV_MSG_TYPE` (u8)
- `2` `TLV_STATUS`   (u8)
- `3` `TLV_SERVICE`  (UTF-8 string)
- `4` `TLV_MODULE`   (UTF-8 string)

### Message Types
Requests:
- `1` `MSG_REGISTER` (service + module)
- `2` `MSG_LOOKUP`   (service)
- `3` `MSG_LIST`     (no fields)

Responses:
- `100` `MSG_ACK`          (status=OK)
- `101` `MSG_LOOKUP_REPLY` (status + module when OK)
- `102` `MSG_LIST_REPLY`   (status + repeated service/module pairs when OK)
- `255` `MSG_ERROR`        (status != OK)

### Status Codes
- `0` OK
- `1` NotFound
- `2` Invalid
- `3` AlreadyExists

---

## 4. Shell Protocol (`ruzzle.shell`)

Purpose: structured control messages for a shell UI.

### TLV Types
- `1` `TLV_MSG_TYPE` (u8)
- `2` `TLV_STATUS`   (u8)
- `3` `TLV_MODULE`   (UTF-8 string)
- `4` `TLV_TOPIC`    (UTF-8 string)
- `5` `TLV_TEXT`     (UTF-8 string)
- `6` `TLV_PATH`     (UTF-8 string)
- `7` `TLV_SLOT`     (UTF-8 string)
- `8` `TLV_USER`     (UTF-8 string)
- `9` `TLV_CONTENT`  (UTF-8 string)
- `10` `TLV_SRC`     (UTF-8 string)
- `11` `TLV_DST`     (UTF-8 string)
- `12` `TLV_FLAG`    (u8)

### Command Types

- `1` `MSG_PS`
- `2` `MSG_LSMOD`
- `3` `MSG_START` (module)
- `4` `MSG_STOP`  (module)
- `5` `MSG_LOG_TAIL`
- `6` `MSG_HELP`  (optional topic)
- `7` `MSG_CATALOG`
- `8` `MSG_INSTALL` (module)
- `9` `MSG_REMOVE` (module)
- `10` `MSG_SETUP`
- `11` `MSG_LOGIN` (user)
- `12` `MSG_LOGOUT`
- `13` `MSG_WHOAMI`
- `14` `MSG_USERS`
- `15` `MSG_USERADD` (user)
- `16` `MSG_PWD`
- `17` `MSG_LS` (optional path)
- `18` `MSG_CD` (path)
- `19` `MSG_MKDIR` (path)
- `20` `MSG_TOUCH` (path)
- `21` `MSG_CAT` (path)
- `22` `MSG_WRITE` (path + content)
- `23` `MSG_EDIT` (path)
- `24` `MSG_CP` (src + dst + flag)
- `25` `MSG_MV` (src + dst)
- `26` `MSG_MKDIRP` (path)
- `27` `MSG_RMR` (path)
- `28` `MSG_SLOTS`
- `29` `MSG_PLUG` (slot + module)
- `30` `MSG_UNPLUG` (slot)
- `31` `MSG_SYSINFO`
- `32` `MSG_RM` (path)

### Response
Responses are text payloads with a status:
- `status=0` OK
- `status=1` Failed

---

## 5. Capability TLV (Negotiation/Metadata)

Capabilities are transferred by the kernel, but modules may still describe
expected or attached caps in their payloads for debugging or policy checks.

### TLV Type
- `50` `TLV_CAP_NAME` (UTF-8 string)

A capability list is encoded as repeated `TLV_CAP_NAME` fields.

---

## 6. Service Naming Rules

To keep the registry deterministic, service names must follow:

```
ruzzle.<segment>(.<segment>)*
segment = [a-z0-9-]+
```

Examples:
- `ruzzle.console`
- `ruzzle.shell`
- `ruzzle.fs.readonly`
