# mosaic-emit-webcomponent

Web Components backend: emits Custom Element classes from MosaicIR

Generated project shells hydrate scalar slots as element properties and
attributes. For `node` and component slots, a host-provided `Element` is mounted
in the custom element's light DOM with the declared named-slot attribute, so a
shadow-DOM `HostSurface` can receive live host content without stringification.
Dynamic scalar and loop values are encoded for their text or quoted-attribute
context before the generated component assigns `shadowRoot.innerHTML`; link
schemes and numeric style interpolation receive additional runtime guards.

`one-of` slots also drive UI49 mosstyle states at runtime. A declaration such
as `state danger` owned by the `variant` slot becomes a conditional CSS layer
based on the generated `variant` attribute local. Base declarations apply
first, enum axes follow model slot order, and existing structural or
interaction `state-when-*` layers remain more specific.

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
