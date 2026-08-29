# Contribuir a Signet

Signet está en `v0.2 · DRAFT`. Las contribuciones son bienvenidas.

## Certificado de origen (DCO)

Cada commit debe llevar la línea `Signed-off-by:` (`git commit -s`). Con eso
certificas que tienes derecho a aportar ese trabajo bajo las licencias del
proyecto: **CC BY 4.0** para `spec/` y **Apache 2.0** para el código.

## Qué se acepta con gusto

- **Implementaciones en otros lenguajes.** Si tu implementación pasa
  `node test-vectors/verify.mjs` (o reproduce `test-vectors/v2.json` byte a
  byte), abre un issue para enlazarla.
- **Vectores de prueba adicionales**, sobre todo de casos límite.
- **Ambigüedades en la especificación.** Si dos personas pueden leer una
  sección de dos formas, eso es un bug del documento.
- Relays que federen entrega entre `relay-host`s distintos.

## Qué requiere discusión previa

Cambiar el **array del sobre firmado** (`signet-msg-v2` o los tokens
`aether-*` en dual-accept) o el perfil `signet-box-v1` rompe la
compatibilidad. Abre un issue antes de escribir código.

## Seguridad

No abras un issue público para una vulnerabilidad. Ver [SECURITY.md](SECURITY.md).
