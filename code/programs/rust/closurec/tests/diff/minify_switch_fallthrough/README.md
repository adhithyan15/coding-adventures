# minify_switch_fallthrough — switch with fall-through cases

Input: `switch(x){case 1:case 2:y();break;default:z();}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `switch(x){case 1:case 2:y();break;default:z()};`

Captured by CLOC14.28 byte-identity exploration.
