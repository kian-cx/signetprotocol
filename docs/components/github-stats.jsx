'use client'

// Estadísticas en vivo del repositorio, leídas de la API pública de GitHub (sin clave).
// Se guardan 10 minutos en el navegador para no agotar el límite de 60 consultas/hora por IP.

import { useEffect, useState } from 'react'

const REPO = 'signetprotocol/signetprotocol'
const URL_REPO = `https://github.com/${REPO}`
const TTL = 10 * 60 * 1000
const CLAVE = 'signet-github-stats'

function leerCache() {
  try {
    const guardado = JSON.parse(localStorage.getItem(CLAVE) || 'null')
    if (guardado && Date.now() - guardado.t < TTL) return guardado.datos
  } catch {}
  return null
}

function guardarCache(datos) {
  try {
    localStorage.setItem(CLAVE, JSON.stringify({ t: Date.now(), datos }))
  } catch {}
}

function useGitHub() {
  const [datos, setDatos] = useState(null)
  useEffect(() => {
    const cache = leerCache()
    if (cache) return setDatos(cache)
    let cancelado = false
    ;(async () => {
      try {
        const [repo, contribuidores] = await Promise.all([
          fetch(`https://api.github.com/repos/${REPO}`).then((r) => (r.ok ? r.json() : Promise.reject(r.status))),
          fetch(`https://api.github.com/repos/${REPO}/contributors?per_page=12`).then((r) => (r.ok ? r.json() : []))
        ])
        const nuevo = {
          estrellas: repo.stargazers_count,
          forks: repo.forks_count,
          incidencias: repo.open_issues_count,
          vigilantes: repo.subscribers_count,
          contribuidores: Array.isArray(contribuidores)
            ? contribuidores.filter((c) => c.type === 'User').map((c) => ({ usuario: c.login, avatar: c.avatar_url, url: c.html_url }))
            : []
        }
        if (!cancelado) {
          guardarCache(nuevo)
          setDatos(nuevo)
        }
      } catch {
        // Sin conexión o límite de la API: los botones siguen funcionando, solo sin cifras.
      }
    })()
    return () => {
      cancelado = true
    }
  }, [])
  return datos
}

function compacto(n) {
  if (n == null) return ''
  return n >= 1000 ? `${(n / 1000).toFixed(n >= 10000 ? 0 : 1).replace(/\.0$/, '')}k` : String(n)
}

const icono = {
  estrella: 'M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.751.751 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z',
  fork: 'M5 5.372v.878c0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75v-.878a2.25 2.25 0 1 1 1.5 0v.878a2.25 2.25 0 0 1-2.25 2.25h-1.5v2.128a2.251 2.251 0 1 1-1.5 0V8.5h-1.5A2.25 2.25 0 0 1 3.5 6.25v-.878a2.25 2.25 0 1 1 1.5 0ZM5 3.25a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Zm6.75.75a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Zm-3 8.75a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Z',
  incidencia: 'M8 9.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM1.5 8a6.5 6.5 0 1 0 13 0 6.5 6.5 0 0 0-13 0Z'
}

function Icono({ d }) {
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
      <path d={d} />
    </svg>
  )
}

/** Botón compacto para la barra superior: ★ Star · 12 */
export function GitHubStars() {
  const datos = useGitHub()
  const n = datos?.estrellas
  return (
    <a className="mv-estrellas" href={URL_REPO} target="_blank" rel="noreferrer" title="Star Signet Protocol on GitHub">
      <Icono d={icono.estrella} />
      <span>Star</span>
      {n > 0 && <b>{compacto(n)}</b>}
    </a>
  )
}

/** Bloque de comunidad para la portada. */
export function GitHubStats() {
  const datos = useGitHub()
  const cifra = (valor, etiqueta, d) => (
    <div className="mv-cifra">
      <span className="mv-cifra-valor">
        <Icono d={d} />
        {valor == null ? '–' : compacto(valor)}
      </span>
      <small>{etiqueta}</small>
    </div>
  )
  return (
    <div className="mv-comunidad">
      <div className="mv-cifras">
        {cifra(datos?.estrellas, 'stars', icono.estrella)}
        {cifra(datos?.forks, 'forks', icono.fork)}
        {cifra(datos?.incidencias, 'open issues', icono.incidencia)}
      </div>
      {datos?.contribuidores?.length > 0 && (
        <div className="mv-avatares" aria-label="Contributors">
          {datos.contribuidores.map((c) => (
            <a key={c.usuario} href={c.url} target="_blank" rel="noreferrer" title={c.usuario}>
              <img src={c.avatar} alt={c.usuario} width="36" height="36" loading="lazy" />
            </a>
          ))}
        </div>
      )}
      <div className="mv-botones">
        <a className="mv-boton principal" href={URL_REPO} target="_blank" rel="noreferrer">Star on GitHub</a>
        <a className="mv-boton" href={`${URL_REPO}/fork`} target="_blank" rel="noreferrer">Fork it</a>
        <a className="mv-boton" href={`${URL_REPO}/discussions`} target="_blank" rel="noreferrer">Join the discussion</a>
        <a className="mv-boton" href="https://x.com/kian_cx" target="_blank" rel="noreferrer">Follow on X</a>
      </div>
    </div>
  )
}
