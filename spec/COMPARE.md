# Signet frente a MCP, A2A y ANP

Este documento fija el recorte de Signet contra los protocolos que un evaluator va a googlear en los primeros cinco minutos. No es un ataque a ninguno de ellos.

## El mapa de capas

| Capa | Pregunta | Protocolo que ya existe | Lo que Signet aporta |
|---|---|---|---|
| Ejecucion intra-autoridad | Como invoca un agente sus herramientas? | **MCP** (Anthropic / Linux Foundation) | Nada. Signet no ejecuta. |
| Coordinacion de tareas | Como dos agentes opacos se delegan un trabajo? | **A2A** v1.0 (Google / Linux Foundation) | Identidad de dueno y consentimiento antes de que A2A transporte la intencion |
| Red abierta de agentes | Como se descubren agentes que no se conocen? | **ANP** (did:wba, handles, discovery, E2E) | Compuerta de contacto + consentimiento de salida + niveles L0-L3 |
| Identidad / mandato / pago | Quien autorizo a este agente a actuar? | AP2, x401, AC2, AIP | Un sobre minimo de mensajeria con consentimiento por-accion |
| Auditoria de tool-calls | Puedo probar que herramienta invoco un agente? | Prismer-AI/signet y otros | Distinto problema. Ver colision de nombre |

Signet v0.2 vive en identidad x consentimiento x entrega. ANP es el vecino mas cercano; la diferencia no es identidad criptografica sino que nada se entrega ni se emite sin arista accepted y consent_token.

Comparativo puntual y lista de proyectos homonimos: ver el cuerpo largo en este archivo en la rama, seccion ANP y seccion Colision de nombre.

## ANP vs Signet v0.2

- ANP identidad: W3C DID did:wba. Signet: Ed25519 + handle federable local@relay (TOFU). DID es extension prevista.
- ANP nombre: alice.example.com via WNS. Signet: alice@relay.example.
- ANP confidencialidad: E2E en el perfil de mensajeria. Signet: signet-box-v1 obligatorio para agent_dm y agent_request.
- ANP consentimiento: no es el primitivo central. Signet: si, en servidor.
- Discovery global: ANP si, Signet v0.2 no.

El sobre Signet puede viajar como payload opaco de un mensaje ANP. Lo inverso tambien vale.

## Colision de nombre

Signet no es un nombre libre en 2026. Proyectos ajenos: Prismer-AI/signet, Maverick0351a/signet-protocol, oleary-labs/signet-protocol, agentic-research/signet, signet.sh, Chroma Signet, Sign Protocol (sign.global).

Este repositorio es el protocolo de mensajeria consentida entre agentes de duenos distintos de Kian Zamorano. Ver NOTICE.
