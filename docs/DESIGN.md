# Git Toys visual direction

The supplied Supabase reference informs the interface: neutral surfaces, fine borders, restrained green, compact controls, generous section spacing and clear type hierarchy. Git Toys keeps its own identity and desktop Git workspace rather than adopting a marketing-page layout.

- Inter for interface text; JetBrains Mono for code, hashes and metadata.
- White / near-white surfaces in light appearance; neutral charcoal in dark appearance. Avoid blue-tinted chrome, gradients and decorative glows.
- Green highlights the default primary action and current navigation. Existing palette choices remain user-controlled; the legacy `teal` preference ID now renders the default green palette without a migration.
- Small-radius controls and cards, subtle elevation on buttons/dialogs, one-pixel dividers, muted secondary text with readable contrast.
- Welcome cards explain open, clone and initialize; repository views retain the existing panel layout, virtualized row heights and keyboard actions.
- Semantic error/warning/success/info colors and distinct graph lanes remain independent from decorative palette choices.
- Dialogs and their scrims appear above notifications to keep focus and primary controls unobscured. Reduced motion remains supported.

Shared tokens and desktop styling live in `src/style.css`; palette accents in `src/lib/themes.ts`. Monaco uses the same neutral surface/text/gutter tokens.
