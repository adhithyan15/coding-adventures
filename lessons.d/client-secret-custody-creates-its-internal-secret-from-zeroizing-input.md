# Client-secret custody creates its internal secret from zeroizing input

`ClientSecretCustody::create` accepts `Zeroizing<String>`, not an already
constructed `ClientSecret`. The custody boundary validates and wraps that
caller-owned value internally. Integration fixtures should pass the zeroizing
string directly instead of calling `ClientSecret::new` first; otherwise the
test does not compile even though both types represent protected secret state.
