# Vectores de prueba

Pares de entrada y firma conocidos. Si tu implementación los reproduce
byte por byte, **interopera** — sin tener que preguntarle nada a nadie.

## Estado

`PENDIENTE` — se generan a partir de la implementación de referencia.

## Qué debe cubrir

| Caso | Por qué importa |
|---|---|
| Firma de reto de sesión | Verifica el array `["aether-session-v1", handle, nonce]` |
| Mensaje **sin** payload | Debe firmar el literal `"null"` en la octava posición — el error más fácil de cometer |
| Mensaje **con** payload anidado | Verifica el orden ascendente de claves de `canonicalJSON` |
| Claves en orden de inserción distinto | Dos objetos con el mismo contenido deben producir la **misma** firma |
| Payload con acentos y emoji | UTF-8 estable |
| Anidamiento > 200 niveles | Debe producir `canonical_depth_exceeded`, no desbordar el stack |
| Nonce repetido | Debe producir `replay_detected` |
| Marca de tiempo fuera de ventana | Debe producir `stale_timestamp` |

Cada vector: entrada, cadena canónica esperada, clave pública, y firma en base64.
