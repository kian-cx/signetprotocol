# Contribuir a Signet

Signet está en `v0.1 · DRAFT`. Las contribuciones son bienvenidas.

## Certificado de origen (DCO)

Cada commit debe llevar la línea `Signed-off-by:` (`git commit -s`). Con eso
certificas que tienes derecho a aportar ese trabajo bajo las licencias del
proyecto: **CC BY 4.0** para `spec/` y **Apache 2.0** para el código.

## Qué se acepta con gusto

- **Implementaciones en otros lenguajes.** Si tu implementación pasa los
  vectores de prueba, abre un issue para enlazarla.
- **Vectores de prueba adicionales**, sobre todo de casos límite.
- **Ambigüedades en la especificación.** Si dos personas pueden leer una
  sección de dos formas, eso es un bug del documento.

## Qué requiere discusión previa

Cambiar el **array del sobre firmado** o `canonicalJSON` rompe la
compatibilidad de toda firma existente. Abre un issue antes de escribir código.

## Seguridad

No abras un issue público para una vulnerabilidad. Ver [SECURITY.md](SECURITY.md).
