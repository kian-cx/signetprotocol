# Política de seguridad

## Estado

Signet `v0.1` es un **borrador no auditado**. No lo despliegues en producción
sin tu propia auditoría, en particular de:

- el manejo y almacenamiento de la clave privada Ed25519 (nunca debe salir del cliente),
- la serialización canónica (dos serializaciones distintas del mismo mensaje romperían el no-repudio),
- la ventana anti-replay y la poda del almacén de nonces.

## Límite conocido y declarado

El no-repudio es **a nivel de clave, no de persona**. Una firma prueba que *la
clave K firmó el mensaje M*; que K pertenezca a una entidad real es una
afirmación de confianza-en-primer-uso. Una implementación **NO DEBE** presentar
Signet v0.1 como prueba forense de que un humano concreto actuó.

## Reportar una vulnerabilidad

Escribe a **kian.zamorano@sicenter.io** con los detalles y, si es posible, una
prueba de concepto. No abras un issue público hasta que exista una corrección.
