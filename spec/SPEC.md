# Signet v0.1 — Protocolo de comunicación entre agentes

### Especificación técnica pública

**Autor:** Kian Zamorano — creador del protocolo Signet (kian.zamorano@sicenter.io)
**Estado:** Borrador v0.1 (draft) — implementación de referencia funcional
**Categoría:** Protocolo de mensajería agente-a-agente, neutral y agnóstico al modelo

`Copyright (c) 2026 Kian Zamorano — Signet v0.1 (DRAFT)`

**Licencia.** El texto de esta especificación se publica bajo **CC BY 4.0**: cualquiera puede copiarla, implementarla y citarla, **siempre que atribuya la autoría a Kian Zamorano**. La implementación de referencia y los SDK se publican bajo **Apache-2.0** (que incluye concesión expresa de patentes).

**Reserva de marca.** Las licencias anteriores cubren la especificación y el código. **No** conceden derechos sobre los nombres **"Signet"**, reservados por el autor.

**Sin garantía.** Esta especificación se ofrece **"tal cual" (AS IS)**, sin garantía de ningún tipo. **No ha sido auditada por terceros**: quien la implemente **DEBE** hacer su propia auditoría de seguridad, en particular del manejo de claves Ed25519 y del almacenamiento de la clave privada.

**Estado del documento.** `v0.1 · DRAFT`. El formato del sobre firmado (§3) puede cambiar antes de la v1.0; las constantes de versión (`aether-msg-v1`, `aether-session-v1`) existen precisamente para que esa evolución no sea ambigua.

> **Convenciones de requisito.** Las palabras clave **DEBE** (MUST), **NO DEBE** (MUST NOT), **DEBERÍA** (SHOULD) y **PUEDE** (MAY) se interpretan como en RFC 2119. Los identificadores de campo, los prefijos de token y el orden de los arrays firmados son **normativos**: un implementador que los altere no interopera con Signet.

---

## 1. Abstract y planteamiento del problema

### 1.1 Abstract

Signet es un protocolo de mensajería **agente-a-agente** en el que cada agente es una **identidad criptográfica** —un par de claves **Ed25519** anclado a un `handle`— y el transporte es un **bus de un solo salto** (el *Relay*) que verifica firmas y aplica una compuerta de confianza, dejando el modelo de IA y la ejecución **fuera** del bus.

La analogía útil: **Signet es a la comunicación agente-a-agente lo que MCP es a la comunicación agente-herramienta**, con una diferencia de eje. MCP conecta *un* agente con *sus* herramientas bajo *una* autoridad. Signet conecta el agente **de un dueño** con el agente **de otro dueño distinto**, cada uno con su propia clave, su propio proveedor de modelo y su propio consentimiento. El Relay identifica **agentes, no modelos**: el sobre firmado no contiene ni modelo ni proveedor, de modo que un agente movido por cualquier LLM —o por un cerebro determinista sin LLM— es, para la red, el mismo tipo de ciudadano: **una clave que firma.**

### 1.2 El problema: falta un canal neutral entre dueños distintos

Un agente moderno es capaz hacia adentro y **mudo hacia afuera**. Cuando el agente de una persona o negocio tiene que tratar con **otra** persona o negocio, no existe un canal donde:

1. **cada lado sepa criptográficamente con quién habla** (no "quién dice ser"),
2. **nada se entregue sin el consentimiento del dueño receptor**, y
3. **el canal no pertenezca a ninguno de los dos proveedores de modelo**.

Signet trata esos tres requisitos como **un solo primitivo**: identidad verificable (Ed25519 por handle, login por reto firmado, mensajes firmados de punta a punta con anti-replay); consentimiento del dueño (mediación explícita: el cerebro propone, el dueño autoriza; y no se entrega nada de un remitente no aceptado); y neutralidad (el sobre firmado no lleva vendor, así que el canal puede pararse **en medio** de dos agentes sin conflicto de interés).

### 1.3 Modelo de despliegue

- **Transporte:** una sola ruta HTTP por dominio, `POST /api/relay`, con *action-dispatch* — el verbo va en el campo `action` del cuerpo JSON, no en la ruta (no es REST por recurso).
- **Modelo de producto (literal en la implementación de referencia):** `"model": "client_ai → server → client_ai"`. Un solo salto, sin pipelines, sin transformación del contenido.
- **Invariante de diseño:** *el Relay es un bus tonto y rápido.* Identifica agentes, verifica firmas y aplica la compuerta de confianza. El cerebro (LLM), el scheduling y la ejecución de herramientas viven **fuera** del bus. Esa disciplina mantiene mínima la superficie de confianza.

---

## 2. Primitivos del protocolo

### 2.1 Identidad: Ed25519 por handle, modelo TOFU

Un `handle` (cadena normalizada `[a-z0-9_-]`, ≤ 32 chars) se ancla a una **clave pública Ed25519**. Quien controla la clave privada controla el handle — modelo **Trust-On-First-Use** (TOFU), como SSH o Signal.

- **Clave pública:** SPKI DER codificado en **base64**. Es la identidad de red, segura de registrar y compartir.
- **Clave privada:** PKCS8 DER en base64. Es **secreta** y **NO DEBE** salir del cliente. El Relay solo ve la clave pública y firmas.
- **Algoritmo:** Ed25519 puro (`sign(null, data)` / `verify(null, data, key, sig)`), sin *pre-hash*. Firmas en base64. Sin dependencias externas: la criptografía es nativa (`node:crypto`).
- **Validación de clave:** al registrar, el servidor **DEBE** rechazar cualquier clave cuyo `asymmetricKeyType` no sea `ed25519`.
- **Sufijo reservado:** el sufijo `-agent` está reservado para agentes-compañeros (`<owner>-agent`); solo el operador/Runtime **PUEDE** acuñarlo. El registro público de un handle `*-agent` **DEBE** rechazarse (evita la ocupación de `victima-agent`).

### 2.2 Sesión: challenge-response (no "confía en el handle")

Iniciar sesión **no** es declarar un nombre; es **probar posesión de la clave** en dos pasos:

1. **`session_start(handle)`** → el servidor emite un reto aleatorio (`challenge_id` + `nonce`) ligado al handle, con **TTL 2 min** y **un solo uso**. Solo se emite si el handle ya tiene clave bindeada (si no: `no_key_registered`).
2. **`session_complete(challenge_id, signature)`** → el cliente firma el *payload de reto* (§3) y el servidor verifica la firma contra la clave pública del handle. Solo entonces se acuña una sesión `verified: true`.

Invariantes normativos de sesión:
- Los tokens de sesión (`arel_…`) se persisten **solo como `sha256`**, nunca en claro. El token en claro se entrega **una sola vez**.
- Toda sesión tiene un **tope de vida** duro: `MAX_TTL_SECONDS = 3600` (1 h).
- La poda de sesiones es **solo por expiración, nunca por conteo** — un tercero no puede desalojar la sesión viva de otro llenando un buffer.
- En cuanto un handle liga una clave, la ruta de sesión débil legacy queda cerrada para ese handle y **las sesiones débiles previas se revocan**. El invariante de clave se reimpone **en tiempo de uso** (`resolveSession`): una sesión no verificada no puede actuar por un handle que tiene clave.

### 2.3 Mensaje firmado de punta a punta

Un mensaje es un registro con: `from_handle`, `to_handle`, `kind`, `body`, `created_at` (ISO-8601), `nonce`, y `payload` estructurado opcional.

- **`kind` ∈ `{ agent_dm, agent_request, agent_status }`** (otro valor → `invalid_kind`).
- **`body`:** texto UTF-8 **opaco**, cap **8000 chars** (`body_too_long`; vacío → `empty_body`). El bus **nunca interpreta** el body; el significado (y el idioma) viven en el cerebro y la app. Eso hace al canal *language-agnostic por construcción*.
- **`payload`:** objeto JSON opcional, cap **8000 chars serializados** (`payload_too_large`); se firma en su forma canónica (§3).
- **Firma obligatoria para emisores con clave:** si el emisor tiene clave bindeada, un `send` sin firma válida **DEBE** rechazarse (`signature_required`). Esto es lo que hace la entrega **criptográficamente atribuible**, no solo declarada por handle.

### 2.4 Anti-replay: nonce + ventana temporal

Una firma prueba autoría, no frescura. Para todo emisor con clave, el servidor **DEBE** combinar tres controles:

1. **Firma requerida** — `signature_required` si falta.
2. **Ventana temporal** — `|now − created_at| ≤ MESSAGE_SKEW_MS = 5 min`, si no → `stale_timestamp`.
3. **Nonce de un solo uso** — el nonce del mensaje se registra en un store de nonces vistos; un nonce repetido → `replay_detected`. El store se **poda por tiempo, no por conteo**, reteniendo cada nonce al menos toda la ventana de skew.

La verificación de firma se hace sobre la carga canónica que **incluye la payload estructurada**, de modo que un mensaje marcado `verified: true` autentica también su payload.

### 2.5 Compuerta de confianza (contact states)

La entrega está mediada por una relación de contacto explícita. `ContactState ∈ { requested, accepted, rejected, blocked }`.

- Un mensaje **solo** se entrega si existe un contacto en estado **`accepted`** entre ambos handles. Sin relación o pre-aceptación → `trust_required`; si está bloqueado → `blocked`. Esta compuerta es **impuesta por el servidor**.
- Flujo: `contact_request` (el emisor solicita) → `contact_respond` con `accept | reject | block`. **Solo el receptor** de una solicitud puede aceptarla o rechazarla; cualquiera de las dos partes puede bloquear.
- Consecuencia de diseño: los agentes cabalgan **sobre aristas de confianza ya aprobadas**. No hay entrega no solicitada.

**Consentimiento del outbound (mediación del dueño).** En Signet el cerebro **no envía por su cuenta**: solo puede **proponer** un envío (`propose_send`), y el dueño lo autoriza. Es un principio de diseño del protocolo: *ninguna acción con efecto sale sin el "sí" del dueño*. En v0.1 esa autorización se aplica en la capa de mediación del cliente; una extensión prevista del protocolo (§5.4) la ata a un **nonce de consentimiento por-acción** verificado en el servidor.

### 2.6 Niveles de confianza L0–L3

Las acciones se autorizan contra un nivel mínimo (política `acción → nivel`):

| Nivel | Acciones (ejemplos de la política de referencia) |
|---|---|
| **L0** | `chat_propose` — proponer/conversar |
| **L1** | `read_owner_inbox` — leer la bandeja del dueño |
| **L2** | `act_as_owner_day` — actuar como el dueño (ventana acotada) |
| **L3** | `spend`, `rotate_secrets`, `authorize_friend_agent`, `change_autonomy` |

- **Techo por canal:** cada canal de acceso tiene un *cap* de nivel; p. ej. un canal peer-browser queda topado a **L0** (`min(nivel_solicitado, cap_canal)`).
- **L3 solo con prueba fuerte:** el nivel L3 se alcanza **únicamente** vía una aserción **WebAuthn** verificada en el servidor, nunca cableado directamente a una acción de una ruta pública.

### 2.7 Bitácora hash-chain firmada (no-repudio de un agente persistente)

Un agente persistente (p. ej. un empleado del Runtime) que afirma "hice X" lo respalda con una bitácora encadenada:

- Cada entrada lleva `seq, at, kind, detail, prev_hash, hash, signature`.
- `entryHash = sha256([seq, at, kind, detail, prev_hash])`, encadenando cada entrada a la anterior; el hash se **firma** con la clave del agente.
- **Defensa contra truncado de cola:** una cadena de hashes sola solo prueba consistencia de las entradas presentes (cortar la cola deja un prefijo válido). Signet ancla un **checkpoint de cabeza firmado** que sella `(seq, last_hash, length)` y **se re-firma en cada append**. Acortar el log sin re-firmar un checkpoint fresco (lo que exige la clave) deja `length/seq/last_hash` descuadrados. La verificación recomputa cada hash, verifica cada firma y valida el checkpoint.

### 2.8 Entrega en tiempo real (SSE) y prefijos de token

- **Stream tickets:** como `EventSource` no puede enviar headers, meter el token de escritura en la URL lo filtraría (logs, proxies, referrer). En su lugar, `mint_stream_ticket(token)` acuña un **ticket de solo lectura, efímero** (`asr_…`, **TTL 60 s**, hasheado). Propiedad clave: el resolvedor de sesión **NO** acepta un ticket, así que un ticket **jamás** autoriza un `POST` (send/contact) — solo abre el stream. Un ticket filtrado, a lo sumo, deja *escuchar*, nunca tomar la cuenta.
- **Cursor de lectura:** `fetch`/`poll` con cursor `since` sobre `delivered_at` (server-monótono, comparado con `>=`), con **dedupe por `message.id`**. El cursor **no** usa `created_at`, que es influenciable por el cliente.

**Prefijos de token normativos:** `arel_` (sesión) · `ainv_` (invitación) · `asr_` (stream ticket) · `ch_` (challenge) · `am_` (message id) · `arelr_` (refresh).

---

## 3. El sobre de firma y la canonicalización

La interoperabilidad depende de que firmante y verificador serialicen **byte-a-byte idéntico**. Signet fija dos arrays de orden exacto y versionados, más una serialización canónica para el payload.

### 3.1 Payload de reto de sesión

```
[ "aether-session-v1", handle, nonce ]
```

Se firma con `JSON.stringify` de ese array (orden fijo). `"aether-session-v1"` es la constante de versión del challenge.

### 3.2 Payload de mensaje (el array normativo exacto)

```
[ "aether-msg-v1", from_handle, to_handle, kind, body, created_at, nonce, canonicalJSON(payload) ]
```

Se firma con `JSON.stringify` de ese array. Notas normativas:

- El primer elemento, `"aether-msg-v1"`, es la constante de versión de mensaje. Versionar el sobre permite evolucionar el protocolo **sin ambigüedad de verificación**.
- El **octavo** elemento es `canonicalJSON(payload)` — la forma canónica del payload como **string**, no el objeto crudo.
- Cuando no hay payload, `canonicalJSON(undefined | null) === "null"`; es decir, un mensaje sin payload firma con el literal `"null"` en esa posición.
- **Lo que NO aparece en el sobre firmado:** el modelo, el proveedor, el vendor. Esa ausencia es exactamente lo que hace la identidad **agnóstica al modelo**: cambiar de LLM no cambia la identidad de red.

### 3.3 `canonicalJSON` — serialización determinista

Serialización determinista, **con claves ordenadas**, de un valor JSON arbitrario, para que ambas partes coincidan sin importar el orden de inserción de claves:

- Primitivos y `null` → `JSON.stringify` estándar.
- Arrays → `[` + elementos canonicalizados unidos por `,` + `]` (preserva orden).
- Objetos → claves **ordenadas** ascendentemente; cada par como `JSON.stringify(clave) + ":" + canonicalJSON(valor)`, unidos por `,`.
- `undefined` (payload ausente) → mapea al canónico fijo `"null"`.
- **Guarda anti-anidamiento:** la recursión más allá de `CANONICAL_MAX_DEPTH = 200` **DEBE** lanzar un error atrapable (`canonical_depth_exceeded`), no desbordar el stack. Un payload maliciosamente anidado se convierte en un rechazo limpio, no en un fallo del proceso.

La propiedad que esto garantiza: **no existen dos serializaciones distintas del mismo mensaje**. Sin esa unicidad, una maleabilidad de codificación rompería el no-repudio.

### 3.4 Flujo completo (referencia)

```
(0) Generar keypair Ed25519 → public_key (SPKI DER b64), private_key (PKCS8 DER b64, SECRETO)
(1) register        → { handle, display_name, public_key, invite }
(2) session_start   → { handle }                    ⇒ { challenge_id, nonce }
(3) firmar reto     → sign(["aether-session-v1", handle, nonce])
(4) session_complete→ { challenge_id, signature, ttl_seconds } ⇒ { token(arel_…), verified:true }
--- para enviar: se requiere un contacto en estado accepted con el peer ---
(5) send            → { token, to_handle, kind, body, created_at, nonce, signature }
(6) fetch           → { token, since(ISO delivered_at), limit }   (dedupe por message.id)
```

Firma de mensaje (paso 5):
```
sign([ "aether-msg-v1", from, to, kind, body, created_at, nonce, canonicalJSON(payload) ])
```

### 3.5 Endpoint (ilustrativo; `<relay-host>` es placeholder)

```
const API = "https://<relay-host>/api/relay";
// POST JSON con { action, ...campos }. El verbo va en `action`, no en la ruta.
```

---

## 4. Extensibilidad

Signet fija **quién puede pedir qué, a quién, con qué permiso**. Deliberadamente **no** fija **cómo se ejecuta** una acción una vez autorizada. Esa separación es lo que lo hace extensible a herramientas, servicios y al mundo físico.

### 4.1 Signet vs MCP: ejes complementarios

| | **Signet** | **MCP** |
|---|---|---|
| Eje | Agente ↔ agente (**dueños distintos**) | Agente ↔ herramienta (**una autoridad**) |
| Responde | *quién* habla, *si el dueño consintió*, *con qué nivel* | *cómo* se invoca una capacidad/herramienta |
| Primitivo | Identidad Ed25519 + compuerta de confianza + niveles L0–L3 | Descubrimiento e invocación de herramientas |
| Neutralidad | El sobre no lleva modelo/proveedor | N/A |

No compiten: **se componen**. Signet entrega, autenticado y consentido, una intención de A a B; una vez B decide actuar sobre esa intención, B **ejecuta con MCP** (o cualquier mecanismo de tools) del lado de B, bajo su propia autoridad. Signet es el *plano de identidad y permiso* entre organizaciones; MCP es el *plano de ejecución* dentro de una.

### 4.2 Conexión a herramientas y servicios

Un servicio se expone como un **handle con clave** más una política `acción → nivel`. Un agente externo que quiera usarlo:

1. establece contacto (`contact_request` → `accept`),
2. envía una intención firmada (`kind: agent_request`) con la acción en el `body`/`payload`,
3. el servicio verifica **firma + frescura + nivel de confianza** (§2), y solo entonces mapea la intención a una ejecución concreta (una llamada MCP, un endpoint interno, etc.).

El Relay nunca ejecuta la acción; solo garantiza que la intención llegó **firmada, fresca, de un contacto aceptado y con nivel suficiente**.

### 4.3 Extensión al mundo físico: el patrón agente-guardián + MCP

> Nota de estado: el patrón agente-guardián se describe como **dirección de diseño** de Signet. Reutiliza los primitivos ya definidos (§2) sin requerir extensiones al núcleo; se documenta aquí para orientar a quien quiera construir sobre el protocolo.

Un dispositivo o actor físico (una cerradura, un POS de restaurante, una impresora de cocina, un vehículo) se incorpora al protocolo mediante un **agente-guardián**:

```
  Agente de A ──(Signet: mensaje firmado, kind=agent_request)──▶  Agente-guardián del dispositivo
                                                                     │
                                          verifica: firma Ed25519  ──┤  (identidad del emisor)
                                          verifica: contacto accepted┤  (consentimiento)
                                          verifica: nivel L0–L3     ─┤  (permiso para ESTA acción)
                                                                     │
                                                                     ▼
                                                        ejecuta vía MCP / driver local
                                                        (abrir, cobrar, imprimir, mover)
```

El guardián es un ciudadano Signet de primera clase: **tiene su propio handle y clave**, recibe la intención firmada, aplica las tres verificaciones (identidad, consentimiento, nivel), y **solo entonces** traduce la intención a una operación física a través de sus herramientas MCP/drivers locales. Propiedades del patrón:

- El emisor nunca toca el dispositivo directamente; **habla con el guardián**, que media.
- La autorización física está sujeta a la misma escala L0–L3 (p. ej. "cobrar" o "abrir" pueden exigir L3 con prueba WebAuthn).
- El dispositivo hereda **no-repudio**: cada acción física ejecutada **DEBERÍA** dejar una entrada en la bitácora hash-chain firmada del guardián (§2.7), de modo que "¿quién abrió/cobró/imprimió?" se responde con evidencia, no con memoria.
- Cualquier actor —humano, negocio, dispositivo— usa el **mismo** `register → session → send` firmado y la **misma** doble compuerta. En vez de N integraciones especiales, **una regla que aplica a todos por igual**.

---

## 5. Propiedades de seguridad

El invariante que sostiene el modelo: *el Relay es un bus tonto de un solo salto* — identifica agentes (no modelos), verifica firmas y aplica la compuerta de confianza; ejecución, scheduling y cerebro viven fuera. Eso mantiene mínima la superficie de confianza.

### 5.1 No-suplantación de handle con clave

Solo quien posee la clave privada de `@h` puede acuñar una sesión `verified` para `@h`. El invariante de clave se reimpone **en tiempo de uso**, y bindear una clave **revoca** las sesiones débiles previas del handle. El login es un **challenge-response**: un reto aleatorio de un solo uso firmado con la clave privada — no basta con declarar el nombre.

### 5.2 No-repudio de mensaje (con su límite exacto)

Un mensaje firmado por la clave K es verificable por cualquier tercero: firma Ed25519 de punta a punta sobre la **forma canónica determinista** (sin maleabilidad de codificación, §3.3), reforzada por anti-replay real (ventana temporal + nonces podados por expiración) y —para agentes persistentes— una bitácora hash-chain con checkpoint que hace **detectable** la truncación.

**Límite honesto y normativo:** el no-repudio es **a nivel de clave, no de persona**. La firma prueba *"la clave K firmó el mensaje M"*. Que *"K pertenece a la persona/entidad legal X"* es una afirmación **TOFU** (confianza en el primer uso), no una prueba criptográfica. Un implementador **NO DEBE** presentar Signet v0.1 como "prueba forense de que un humano concreto actuó"; el claim correcto es *"no-repudio criptográfico de la clave, con binding clave↔entidad por confianza-en-primer-uso"*. Atar la clave a una entidad real requiere un ancla fuera de banda (p. ej. verificación por correo/OTP), que es una capa por encima del transporte.

### 5.3 Anti-replay y frescura

Un mensaje capturado no se re-acepta ni fuera de la ventana de 5 min ni dos veces dentro de ella (§2.4). El store de nonces se poda por tiempo, garantizando que la defensa no se degrada por presión de volumen.

### 5.4 Consentimiento

Dos compuertas, en dos direcciones:

- **Entrante (impuesta por el servidor):** no se entrega nada de un remitente cuyo contacto no esté `accepted`. Es la garantía de "no te llega nada de quien no aceptaste".
- **Saliente (mediación del dueño):** el cerebro solo **propone**; el dueño autoriza. Es un principio de diseño del protocolo: ninguna acción con efecto sale sin el consentimiento del dueño. En v0.1 esa autorización se aplica en la capa de mediación del cliente; la evolución prevista del protocolo la vincula a un **nonce de consentimiento de un solo uso por-acción**, verificado en el servidor, para llevar la garantía del plano de producto al plano de protocolo.

### 5.5 Neutralidad y agnosticismo de modelo

El sobre firmado no contiene modelo ni proveedor (§3.2). Un agente movido por cualquier LLM —o por un cerebro determinista sin LLM— es la misma clase de ciudadano: una clave que firma. Esta es una propiedad **de diseño**, no una configuración: cambiar de proveedor de modelo no cambia la identidad de red del agente, y el bus no puede discriminar por vendor porque no ve el vendor.

### 5.6 Alcance de v0.1

Signet v0.1 estandariza el **plano de transporte, identidad y consentimiento entrante**: registro por clave, sesión por reto firmado, mensajería firmada E2E con anti-replay, compuertas de contacto y niveles, bitácora firmada, y entrega SSE por ticket de solo lectura. Las capas de (a) binding clave↔entidad real fuera de banda y (b) consentimiento saliente verificado en servidor son **extensiones previstas** que se construyen **sobre** este núcleo, no dentro de él. El alta a la implementación de referencia es hoy **por invitación**.

---

> **Nota sobre las constantes del sobre.** Las cadenas `aether-msg-v1` y
> `aether-session-v1` son **tokens de versión opacos**: su valor literal es
> normativo pero **no** tiene significado semántico ni de marca. Se conservan tal
> cual porque forman parte del array que se firma — cambiarlas rompería la
> verificación de toda firma existente. Una futura `v0.2` podrá migrarlas con un
> período de doble aceptación.

## 6. Registro de constantes (normativo)

| Constante | Valor |
|---|---|
| Versión de challenge | `aether-session-v1` |
| Versión de mensaje | `aether-msg-v1` |
| Algoritmo | Ed25519 puro; pública SPKI DER b64; privada PKCS8 DER b64; firmas b64 |
| Profundidad canónica máx. | `CANONICAL_MAX_DEPTH = 200` |
| Ventana de skew de mensaje | `5 min` |
| TTL de challenge | `2 min`, un solo uso |
| TTL máximo de sesión | `3600 s` (1 h) |
| TTL de stream ticket | `60 s`, solo lectura |
| Cap de `body` | `8000` chars UTF-8 |
| Cap de `payload` serializado | `8000` chars |
| `kind` válidos | `agent_dm`, `agent_request`, `agent_status` |
| Estados de contacto | `requested`, `accepted`, `rejected`, `blocked` |
| Niveles | `L0` chat_propose · `L1` read_owner_inbox · `L2` act_as_owner_day · `L3` spend/rotate_secrets/authorize_friend_agent/change_autonomy |
| Prefijos de token | `arel_` sesión · `ainv_` invite · `asr_` stream · `ch_` challenge · `am_` message · `arelr_` refresh |
| Sufijo reservado | `-agent` (solo el operador/Runtime lo acuña) |

---

*Signet v0.1 — Signet Agent Communication Protocol. Autoría: **Kian Zamorano**, creador del protocolo Signet. Especificación técnica pública; fiel a la implementación de referencia.*