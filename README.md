# Signet

**Un protocolo abierto para que el agente de una persona o negocio pueda hablar con el agente de otra** — con identidad criptográfica, consentimiento del dueño, y sin que el canal pertenezca a ningún proveedor de modelos.

> Los agentes de IA son brillantes hacia adentro y **mudos hacia afuera**. Signet les da el canal que les falta.

**Estado:** `v0.1 · DRAFT` — especificación estable en su núcleo, sujeta a cambios antes de la v1.0.

---

## No compite con MCP ni con A2A

Signet opera en una **capa distinta** y está diseñado para apoyarse en ellos.

| Protocolo | Responde | Relación con Signet |
|---|---|---|
| **MCP** | *Cómo* un agente invoca una herramienta, bajo **una** autoridad | Se usa **después** de que llega una intención consentida |
| **A2A** | *Cómo* dos agentes coordinan una tarea | Puede transportar la intención; Signet aporta **de quién es** y **si consintió** |
| **Signet** | *Quién* habla, *si el dueño aceptó*, *con qué nivel* | — |

Nada de lo anterior deja de funcionar si adoptas Signet, y Signet no te obliga a abandonar nada. **El sobre firmado no contiene modelo, proveedor ni marca** — precisamente para poder viajar dentro de la infraestructura de otros.

---

## Los cinco primitivos

1. **Identidad** — un `handle` anclado a una clave pública Ed25519. La privada nunca sale del cliente.
2. **Sesión** — no declaras un nombre: **firmas un reto** de un solo uso.
3. **Mensaje firmado** — de punta a punta, sobre una forma canónica determinista, con ventana temporal y nonce.
4. **Compuerta de confianza** — solo se entrega si existe un contacto **aceptado**. La puerta se abre desde adentro.
5. **Bitácora firmada** — cadena de hashes con checkpoint de cabeza: borrar el final es **detectable**.

## El sobre que se firma

```
sign([ "aether-msg-v1", from, to, kind, body,
       created_at, nonce, canonicalJSON(payload) ])
```

El orden es **normativo**. Cambiarlo rompe la interoperabilidad aunque la criptografía sea correcta.

> Las cadenas `aether-msg-v1` / `aether-session-v1` son **tokens de versión opacos**, sin significado de marca. Se conservan porque forman parte del array firmado.

---

## Empieza

- **[Especificación completa](spec/SPEC.md)** — RFC 2119, con sus límites declarados.
- **[Cliente de referencia](reference/signet-client.mjs)** — el protocolo completo en **85 líneas, sin una sola dependencia externa**.
- **[Vectores de prueba](test-vectors/)** — para implementar en cualquier lenguaje sin preguntarle nada a nadie.

```
(0) Genera un par de claves Ed25519
(1) register         → reclama tu handle con la clave pública
(2) session_start    → el servidor te devuelve un reto
(3) firma el reto
(4) session_complete → sesión verificada
(5) send             → mensaje firmado (requiere contacto aceptado)
(6) fetch            → recibe, con cursor y sin duplicados
```

---

## Lo que Signet NO hace

- **No prueba que una persona concreta actuó.** Prueba que *la clave K firmó el mensaje M*. Atar esa clave a una persona real es confianza-en-primer-uso, no criptografía.
- **No ejecuta nada.** Entrega intenciones; quien las recibe decide si actúa.
- **No interpreta el contenido.** El cuerpo es texto opaco.

---

## Licencias

- **Especificación** (`spec/`) — [CC BY 4.0](LICENSE-SPEC): libre de copiar e implementar **con atribución al autor**.
- **Código** (`reference/`) — [Apache 2.0](LICENSE): incluye concesión expresa de patentes.

Las licencias **no** conceden derechos sobre la marca **Signet**. Ver [NOTICE](NOTICE).

**Autor:** Kian Zamorano — creador del protocolo Signet.
