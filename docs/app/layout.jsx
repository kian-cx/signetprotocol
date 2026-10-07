import { Footer, Layout, Navbar, ThemeSwitch } from 'nextra-theme-docs'
import { Banner, Head, Search } from 'nextra/components'
import { GitHubStars } from '../components/github-stats'
import { VisitorCounter } from '../components/visitas'
import { getPageMap } from 'nextra/page-map'
import 'nextra-theme-docs/style.css'
import './estilos.css'

export const metadata = {
  title: {
    default: 'Signet Protocol',
    template: '%s · Signet Protocol'
  },
  description: 'Every game, one world: an open protocol and SDK so different games can share a server, a world and a match.'
}

const logo = (
  <span className="mv-logo">
    <b>Signet</b>
    <span className="mv-logo-sdk">PROTOCOL</span>
  </span>
)

export default async function RootLayout({ children }) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      {/* Monocromo: sin tono ni saturación, solo negro, blanco y grises. */}
      <Head
        color={{ hue: 0, saturation: 0, lightness: { dark: 92, light: 8 } }}
        backgroundColor={{ dark: 'rgb(10,10,10)', light: 'rgb(255,255,255)' }}
      />
      <body>
        <Layout
          banner={<Banner storageKey="signet-beta-1">Signet SDK 0.1 is in beta: the API may change. Your translators are welcome!</Banner>}
          navbar={
            <Navbar logo={logo} projectLink="https://github.com/signetprotocol/signetprotocol">
              {/* Estrellas de GitHub en vivo y selector claro / oscuro / sistema (visibles también en el móvil). */}
              <GitHubStars />
              <ThemeSwitch lite />
            </Navbar>
          }
          pageMap={await getPageMap()}
          footer={
            <Footer>
              <div className="mv-pie">
                <span>Apache-2.0 · {new Date().getFullYear()} · Signet Protocol. Games and their trademarks belong to their owners.</span>
                <VisitorCounter />
              </div>
            </Footer>
          }
          sidebar={{ defaultMenuCollapseLevel: 1 }}
          editLink={null}
          feedback={{ content: null }}
          toc={{ title: 'On this page', backToTop: 'Back to top' }}
          search={<Search placeholder="Search the docs…" emptyResult="No results." errorText="Could not load the search index." loading="Loading…" />}
          copyPageButton={false}
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
