# sandhi: a server cannot receive a chunked request body (501)

**Found by:** agnosai (2026-11-05, roadmap B16 re-check). **Version:** sandhi 1.13.7.

`sandhi_server_request_has_unsupported_te` returns 1 for any `Transfer-Encoding` header, and every server
loop answers 501 before the handler runs: `run_opts`, the async handler, the pooled worker and TLS. A probe
on `run_pooled`:

```cyr
include "lib/syscalls.cyr"
include "lib/alloc.cyr"
include "lib/sandhi.cyr"

fn h(ctx, cfd, buf, blen) {
    var off = sandhi_server_body_offset(buf, blen);
    var bl = blen - off;
    var sb = str_builder_new();
    str_builder_add_cstr(sb, "handler saw body bytes=");
    str_builder_add_int(sb, bl);
    var s = str_builder_build(sb);
    sandhi_server_send_response(cfd, 200, "OK", "text/plain", str_data(s), str_len(s), 0);
    return 0;
}

fn main() {
    alloc_init();
    _w_unused();
    return sandhi_server_run_pooled(INADDR_LOOPBACK(), 18931, &h, 0, sandhi_server_options_new());
}
fn _w_unused() { return 0; }
var r = main();
syscall(60, 0);
```

`curl --data-binary` with `Content-Length` gets 200 and "handler saw body bytes=5".
With `-H 'Transfer-Encoding: chunked'` it gets `501 Not Implemented`, and the handler is never called.

**Ask:** decode `chunked` request bodies, under the existing `max_request` cap and request deadline.
sandhi's CHANGELOG 1.9.4 recorded decoding as "not part of this patch". **agnosai** answers such a client 501
today (its roadmap *Out of scope*, and B8).
