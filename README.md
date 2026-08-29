# Signet

**Un protocolo abierto para que el agente de una persona o negocio pueda hablar con el agente de otra** — con identidad criptográfica, consentimiento del dueño *en el servidor*, body cifrado frente al Relay, y un nombre que no queda preso de un registry plano.

> Los agentes de IA son brillantes hacia adentro y **mudos hacia afuera**. Signet les da el canal que les falta.

**Estado:** `v0.2 · DRAFT` — endurece v0.1. Sujeto a cambios antes de la v1.0.

---

## No compite con MCP, A2A ni ANP

Signet opera en una **capa distinta**. Comparativo largo: [spec/COMPARE.md](spec/COMPARE.md).

| Protocolo | Responde | Relación con Signet |
|---|---|---|
| **MCP** | *Cómo* un agente invoca una herramienta, bajo **una** autoridad | Se usa **después** de que llega una intención consentida |
| **A2A** | *Cómo* dos agentes coordinan una tarea | Puede transportar la intención; Signet aporta **de quién es** y **si consintió** |
| **ANP** | Identidad DID + discovery + mensajería en red abierta | Vecino más cercano. Signet es más estrecho: consentimiento y entrega |
| **Signet** | *Quién* habla, *si el dueño aceptó a ambos lados*, *con qué nivel* | — |

El sobre firmado **no contiene modelo, proveedor ni marca**.

---

## Los primitivos (v0.2)

1. **Identidad federable** — `alice@relay.example` anclada a Ed25519 (firma) + X25519 (cifrado). TOFU. La privada no sale del cliente.
2. **Sesión** — firmas un reto de un solo uso, no declaras un nombre.
3. **Mensaje firmado + cifrado** — JCS (RFC 8785), ventana temporal, nonce CSPRNG, `signet-box-v1` para DM y request.
4. **Doble compuerta** — entrada: contacto `accepted`. Salida: `consent_token` de un solo uso verificado en el servidor, atado al `body_sha256`.
5. **Bitácora firmada** — cadena de hashes con checkpoint de cabeza.
6. **Ciclo de vida** — rotación, revocación, delegación con techo L2.

## El sobre v2

```
sign([ "signet-msg-v2", from, to, kind, created_at, nonce,
       jcs(payload), body_sha256, jcs(box), consent_id ])
```

`aether-msg-v1` se acepta en ventana de dual-accept y queda deprecado. El orden es normativo.

---

## Empieza

- **[Especificación](spec/SPEC.md)** — RFC 2119, con límites declarados.
- **[Contrato del Relay](spec/RELAY.md)** — cada `action` y cada código de error.
- **[Comparativo](spec/COMPARE.md)** — MCP / A2A / ANP / colisión de nombre.
- **[Cliente](reference/signet-client.mjs)** y **[Relay](reference/signet-relay.mjs)** de referencia, cero dependencias externas.
- **[Vectores de prueba](test-vectors/v2.json)** — firmas, JCS y box reproducibles.

```
node reference/signet-relay.mjs          # http://localhost:8787
node reference/signet-client.mjs register --handle alice
node reference/signet-client.mjs register --handle bob
node reference/signet-client.mjs trust    --handle alice --to bob
node reference/signet-client.mjs accept   --handle bob --from alice
node reference/signet-client.mjs send     --handle alice --to bob --body "hola"
node reference/signet-client.mjs poll     --handle bob
node test-vectors/verify.mjs
```

---

## Lo que Signet NO hace

- **No prueba que una persona concreta actuó.** Prueba que *la clave K firmó el sobre M*.
- **No ejecuta nada.** Entrega intenciones.
- **No interpreta el contenido.** El body es opaco.
- **No es una red P2P todavía.** El formato `local@relay` es obligatorio; el reenvío inter-relay es OPTIONAL.
- **No es MLS.** El box es X25519+ChaCha20-Poly1305 sin forward secrecy.

---

## Licencias

- **Especificación** (`spec/`) — [CC BY 4.0](LICENSE-SPEC).
- **Código** (`reference/`, `test-vectors/*.mjs`) — [Apache 2.0](LICENSE).

Las licencias **no** conceden derechos sobre la marca **Signet**. Ver [NOTICE](NOTICE).

**Autor:** Kian Zamorano — creador del protocolo Signet.
