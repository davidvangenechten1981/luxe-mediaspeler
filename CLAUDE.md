# Instructies voor Claude (VLC Ultimate)

Lees eerst **`docs/OVERDRACHT.md`**: daar staat alles over doel, architectuur, API-regels, tests, CI, valkuilen en werkwijze.

Kort:
- Eigenaar David: Nederlandstalig, beginner. **Antwoord in het Nederlands, kort en stap voor stap.**
- Werk op branch `studio`; voeg alleen samen met `main` als David dat uitdrukkelijk vraagt.
- Test eerst (TDD). Draai vóór het pushen: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm typecheck && pnpm lint && pnpm test`.
- Nieuwe API-verzoeken: `api.rs` + `api_names.txt` + `src/api/client.ts` + `src/api/types.ts` (+ golden/contracttest bij gewijzigde structuren).
- Programma's starten alleen via `vu_core::proc::quiet()` of `background()`.
- Geen internetfuncties of open poorten zonder uitdrukkelijk verzoek. Wijzig nooit de bestanden van de gebruiker zonder reservekopie of tijdelijk bestand.
- Wees eerlijk over wat niet op een echte Windows-pc getest kon worden.
