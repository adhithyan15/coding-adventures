# mosaic-emit-webcomponent

Web Components backend: emits Custom Element classes from MosaicIR

Generated project shells hydrate scalar slots as element properties and
attributes. For `node` and component slots, a host-provided `Element` is mounted
in the custom element's light DOM with the declared named-slot attribute, so a
shadow-DOM `HostSurface` can receive live host content without stringification.

## Dependencies

- mosaic-vm
- mosaic-analyzer
- mosaic-parser
- mosaic-lexer
- grammar-tools
- lexer
- directed-graph
- parser
- state-machine

## Development

```bash
# Run tests
bash BUILD
```
