/**
 * Keep the two diagnostic inputs in one lazy chunk without widening the group
 * to the authored shard files behind their virtual modules.
 *
 * The browser sees one reconstructed chapter-capability module and one
 * reconstructed book-hash module per track. It must never see one module per
 * generated chapter owner: that would turn today's 1,088 records into 1,088
 * requests and make the bundle shape grow with chapters instead of tracks.
 */
export function isBookLedgerModuleId(moduleId: string): boolean {
  const normalized = moduleId.replaceAll("\\", "/").split(/[?#]/, 1)[0];
  return /^(?:\0)?virtual:human-language-ledger\/(?:chapters|book-hashes)\/[a-z][a-z0-9-]*$/.test(
    normalized,
  );
}
