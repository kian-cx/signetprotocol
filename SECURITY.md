# Política de seguridad

## Estado

Signet `v0.2` es un **borrador no auditado**. No lo despliegues en producción
sin tu propia auditoría, en particular de:

- el manejo y almacenamiento de las claves privadas Ed25519 y X25519
  (nunca deben salir del cliente; `SIGNET_PASS` cifra el JSON en disco en la
  referencia, no sustituye un HSM ni un OS keystore),
- la serialización JCS (RFC 8785) — dos serializaciones distintas del mismo
  mensaje romperían el no-repudio,
- la ventana anti-replay y la poda del almacén de nonces,
- el consumo atómico del `consent_token` (un crash entre verificar y marcar
  `spent` no debe permitir replay),
- el box `x25519-chacha20poly1305-v1` (AAD atada a from/to/created_at/nonce).

## Límites conocidos y declarados

- El no-repudio es **a nivel de clave, no de persona**.
- El Relay ve metadatos (quién, a quién, cuándo, kind, tamaño). No ve el
  plaintext de `agent_dm` / `agent_request` v2.
- No hay forward secrecy ni post-compromise security.
- Perder la privada sin haber rotado es perder la dirección. No hay recuperación
  por correo en el núcleo.
- Una delegación no puede otorgar L3. L3 exige WebAuthn en el servidor; la
  implementación de referencia **aún no verifica WebAuthn** y por eso **NO DEBE**
  honrar grants L3.

## Reportar una vulnerabilidad

Escribe a **kian.zamorano@sicenter.io** con los detalles y, si es posible, una
prueba de concepto. No abras un issue público hasta que exista una corrección.
