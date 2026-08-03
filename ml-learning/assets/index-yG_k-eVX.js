var e=(e,t)=>()=>(t||(e((t={exports:{}}).exports,t),e=null),t.exports);(function(){let e=document.createElement(`link`).relList;if(e&&e.supports&&e.supports(`modulepreload`))return;for(let e of document.querySelectorAll(`link[rel="modulepreload"]`))n(e);new MutationObserver(e=>{for(let t of e)if(t.type===`childList`)for(let e of t.addedNodes)e.tagName===`LINK`&&e.rel===`modulepreload`&&n(e)}).observe(document,{childList:!0,subtree:!0});function t(e){let t={};return e.integrity&&(t.integrity=e.integrity),e.referrerPolicy&&(t.referrerPolicy=e.referrerPolicy),e.crossOrigin===`use-credentials`?t.credentials=`include`:e.crossOrigin===`anonymous`?t.credentials=`omit`:t.credentials=`same-origin`,t}function n(e){if(e.ep)return;e.ep=!0;let n=t(e);fetch(e.href,n)}})();var t=e((e=>{var t=Symbol.for(`react.transitional.element`),n=Symbol.for(`react.portal`),r=Symbol.for(`react.fragment`),i=Symbol.for(`react.strict_mode`),a=Symbol.for(`react.profiler`),o=Symbol.for(`react.consumer`),s=Symbol.for(`react.context`),c=Symbol.for(`react.forward_ref`),l=Symbol.for(`react.suspense`),u=Symbol.for(`react.memo`),d=Symbol.for(`react.lazy`),f=Symbol.for(`react.activity`),p=Symbol.iterator;function m(e){return typeof e!=`object`||!e?null:(e=p&&e[p]||e[`@@iterator`],typeof e==`function`?e:null)}var h={isMounted:function(){return!1},enqueueForceUpdate:function(){},enqueueReplaceState:function(){},enqueueSetState:function(){}},g=Object.assign,_={};function v(e,t,n){this.props=e,this.context=t,this.refs=_,this.updater=n||h}v.prototype.isReactComponent={},v.prototype.setState=function(e,t){if(typeof e!=`object`&&typeof e!=`function`&&e!=null)throw Error(`takes an object of state variables to update or a function which returns an object of state variables.`);this.updater.enqueueSetState(this,e,t,`setState`)},v.prototype.forceUpdate=function(e){this.updater.enqueueForceUpdate(this,e,`forceUpdate`)};function y(){}y.prototype=v.prototype;function b(e,t,n){this.props=e,this.context=t,this.refs=_,this.updater=n||h}var x=b.prototype=new y;x.constructor=b,g(x,v.prototype),x.isPureReactComponent=!0;var ee=Array.isArray;function S(){}var C={H:null,A:null,T:null,S:null},te=Object.prototype.hasOwnProperty;function ne(e,n,r){var i=r.ref;return{$$typeof:t,type:e,key:n,ref:i===void 0?null:i,props:r}}function w(e,t){return ne(e.type,t,e.props)}function re(e){return typeof e==`object`&&!!e&&e.$$typeof===t}function ie(e){var t={"=":`=0`,":":`=2`};return`$`+e.replace(/[=:]/g,function(e){return t[e]})}var ae=/\/+/g;function T(e,t){return typeof e==`object`&&e&&e.key!=null?ie(``+e.key):t.toString(36)}function oe(e){switch(e.status){case`fulfilled`:return e.value;case`rejected`:throw e.reason;default:switch(typeof e.status==`string`?e.then(S,S):(e.status=`pending`,e.then(function(t){e.status===`pending`&&(e.status=`fulfilled`,e.value=t)},function(t){e.status===`pending`&&(e.status=`rejected`,e.reason=t)})),e.status){case`fulfilled`:return e.value;case`rejected`:throw e.reason}}throw e}function se(e,r,i,a,o){var s=typeof e;(s===`undefined`||s===`boolean`)&&(e=null);var c=!1;if(e===null)c=!0;else switch(s){case`bigint`:case`string`:case`number`:c=!0;break;case`object`:switch(e.$$typeof){case t:case n:c=!0;break;case d:return c=e._init,se(c(e._payload),r,i,a,o)}}if(c)return o=o(e),c=a===``?`.`+T(e,0):a,ee(o)?(i=``,c!=null&&(i=c.replace(ae,`$&/`)+`/`),se(o,r,i,``,function(e){return e})):o!=null&&(re(o)&&(o=w(o,i+(o.key==null||e&&e.key===o.key?``:(``+o.key).replace(ae,`$&/`)+`/`)+c)),r.push(o)),1;c=0;var l=a===``?`.`:a+`:`;if(ee(e))for(var u=0;u<e.length;u++)a=e[u],s=l+T(a,u),c+=se(a,r,i,s,o);else if(u=m(e),typeof u==`function`)for(e=u.call(e),u=0;!(a=e.next()).done;)a=a.value,s=l+T(a,u++),c+=se(a,r,i,s,o);else if(s===`object`){if(typeof e.then==`function`)return se(oe(e),r,i,a,o);throw r=String(e),Error(`Objects are not valid as a React child (found: `+(r===`[object Object]`?`object with keys {`+Object.keys(e).join(`, `)+`}`:r)+`). If you meant to render a collection of children, use an array instead.`)}return c}function ce(e,t,n){if(e==null)return e;var r=[],i=0;return se(e,r,``,``,function(e){return t.call(n,e,i++)}),r}function le(e){if(e._status===-1){var t=e._result;t=t(),t.then(function(t){(e._status===0||e._status===-1)&&(e._status=1,e._result=t)},function(t){(e._status===0||e._status===-1)&&(e._status=2,e._result=t)}),e._status===-1&&(e._status=0,e._result=t)}if(e._status===1)return e._result.default;throw e._result}var E=typeof reportError==`function`?reportError:function(e){if(typeof window==`object`&&typeof window.ErrorEvent==`function`){var t=new window.ErrorEvent(`error`,{bubbles:!0,cancelable:!0,message:typeof e==`object`&&e&&typeof e.message==`string`?String(e.message):String(e),error:e});if(!window.dispatchEvent(t))return}else if(typeof process==`object`&&typeof process.emit==`function`){process.emit(`uncaughtException`,e);return}console.error(e)},D={map:ce,forEach:function(e,t,n){ce(e,function(){t.apply(this,arguments)},n)},count:function(e){var t=0;return ce(e,function(){t++}),t},toArray:function(e){return ce(e,function(e){return e})||[]},only:function(e){if(!re(e))throw Error(`React.Children.only expected to receive a single React element child.`);return e}};e.Activity=f,e.Children=D,e.Component=v,e.Fragment=r,e.Profiler=a,e.PureComponent=b,e.StrictMode=i,e.Suspense=l,e.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=C,e.__COMPILER_RUNTIME={__proto__:null,c:function(e){return C.H.useMemoCache(e)}},e.cache=function(e){return function(){return e.apply(null,arguments)}},e.cacheSignal=function(){return null},e.cloneElement=function(e,t,n){if(e==null)throw Error(`The argument must be a React element, but you passed `+e+`.`);var r=g({},e.props),i=e.key;if(t!=null)for(a in t.key!==void 0&&(i=``+t.key),t)!te.call(t,a)||a===`key`||a===`__self`||a===`__source`||a===`ref`&&t.ref===void 0||(r[a]=t[a]);var a=arguments.length-2;if(a===1)r.children=n;else if(1<a){for(var o=Array(a),s=0;s<a;s++)o[s]=arguments[s+2];r.children=o}return ne(e.type,i,r)},e.createContext=function(e){return e={$$typeof:s,_currentValue:e,_currentValue2:e,_threadCount:0,Provider:null,Consumer:null},e.Provider=e,e.Consumer={$$typeof:o,_context:e},e},e.createElement=function(e,t,n){var r,i={},a=null;if(t!=null)for(r in t.key!==void 0&&(a=``+t.key),t)te.call(t,r)&&r!==`key`&&r!==`__self`&&r!==`__source`&&(i[r]=t[r]);var o=arguments.length-2;if(o===1)i.children=n;else if(1<o){for(var s=Array(o),c=0;c<o;c++)s[c]=arguments[c+2];i.children=s}if(e&&e.defaultProps)for(r in o=e.defaultProps,o)i[r]===void 0&&(i[r]=o[r]);return ne(e,a,i)},e.createRef=function(){return{current:null}},e.forwardRef=function(e){return{$$typeof:c,render:e}},e.isValidElement=re,e.lazy=function(e){return{$$typeof:d,_payload:{_status:-1,_result:e},_init:le}},e.memo=function(e,t){return{$$typeof:u,type:e,compare:t===void 0?null:t}},e.startTransition=function(e){var t=C.T,n={};C.T=n;try{var r=e(),i=C.S;i!==null&&i(n,r),typeof r==`object`&&r&&typeof r.then==`function`&&r.then(S,E)}catch(e){E(e)}finally{t!==null&&n.types!==null&&(t.types=n.types),C.T=t}},e.unstable_useCacheRefresh=function(){return C.H.useCacheRefresh()},e.use=function(e){return C.H.use(e)},e.useActionState=function(e,t,n){return C.H.useActionState(e,t,n)},e.useCallback=function(e,t){return C.H.useCallback(e,t)},e.useContext=function(e){return C.H.useContext(e)},e.useDebugValue=function(){},e.useDeferredValue=function(e,t){return C.H.useDeferredValue(e,t)},e.useEffect=function(e,t){return C.H.useEffect(e,t)},e.useEffectEvent=function(e){return C.H.useEffectEvent(e)},e.useId=function(){return C.H.useId()},e.useImperativeHandle=function(e,t,n){return C.H.useImperativeHandle(e,t,n)},e.useInsertionEffect=function(e,t){return C.H.useInsertionEffect(e,t)},e.useLayoutEffect=function(e,t){return C.H.useLayoutEffect(e,t)},e.useMemo=function(e,t){return C.H.useMemo(e,t)},e.useOptimistic=function(e,t){return C.H.useOptimistic(e,t)},e.useReducer=function(e,t,n){return C.H.useReducer(e,t,n)},e.useRef=function(e){return C.H.useRef(e)},e.useState=function(e){return C.H.useState(e)},e.useSyncExternalStore=function(e,t,n){return C.H.useSyncExternalStore(e,t,n)},e.useTransition=function(){return C.H.useTransition()},e.version=`19.2.7`})),n=e(((e,n)=>{n.exports=t()})),r=e((e=>{function t(e,t){var n=e.length;e.push(t);a:for(;0<n;){var r=n-1>>>1,a=e[r];if(0<i(a,t))e[r]=t,e[n]=a,n=r;else break a}}function n(e){return e.length===0?null:e[0]}function r(e){if(e.length===0)return null;var t=e[0],n=e.pop();if(n!==t){e[0]=n;a:for(var r=0,a=e.length,o=a>>>1;r<o;){var s=2*(r+1)-1,c=e[s],l=s+1,u=e[l];if(0>i(c,n))l<a&&0>i(u,c)?(e[r]=u,e[l]=n,r=l):(e[r]=c,e[s]=n,r=s);else if(l<a&&0>i(u,n))e[r]=u,e[l]=n,r=l;else break a}}return t}function i(e,t){var n=e.sortIndex-t.sortIndex;return n===0?e.id-t.id:n}if(e.unstable_now=void 0,typeof performance==`object`&&typeof performance.now==`function`){var a=performance;e.unstable_now=function(){return a.now()}}else{var o=Date,s=o.now();e.unstable_now=function(){return o.now()-s}}var c=[],l=[],u=1,d=null,f=3,p=!1,m=!1,h=!1,g=!1,_=typeof setTimeout==`function`?setTimeout:null,v=typeof clearTimeout==`function`?clearTimeout:null,y=typeof setImmediate<`u`?setImmediate:null;function b(e){for(var i=n(l);i!==null;){if(i.callback===null)r(l);else if(i.startTime<=e)r(l),i.sortIndex=i.expirationTime,t(c,i);else break;i=n(l)}}function x(e){if(h=!1,b(e),!m)if(n(c)!==null)m=!0,ee||(ee=!0,re());else{var t=n(l);t!==null&&T(x,t.startTime-e)}}var ee=!1,S=-1,C=5,te=-1;function ne(){return g?!0:!(e.unstable_now()-te<C)}function w(){if(g=!1,ee){var t=e.unstable_now();te=t;var i=!0;try{a:{m=!1,h&&(h=!1,v(S),S=-1),p=!0;var a=f;try{b:{for(b(t),d=n(c);d!==null&&!(d.expirationTime>t&&ne());){var o=d.callback;if(typeof o==`function`){d.callback=null,f=d.priorityLevel;var s=o(d.expirationTime<=t);if(t=e.unstable_now(),typeof s==`function`){d.callback=s,b(t),i=!0;break b}d===n(c)&&r(c),b(t)}else r(c);d=n(c)}if(d!==null)i=!0;else{var u=n(l);u!==null&&T(x,u.startTime-t),i=!1}}break a}finally{d=null,f=a,p=!1}i=void 0}}finally{i?re():ee=!1}}}var re;if(typeof y==`function`)re=function(){y(w)};else if(typeof MessageChannel<`u`){var ie=new MessageChannel,ae=ie.port2;ie.port1.onmessage=w,re=function(){ae.postMessage(null)}}else re=function(){_(w,0)};function T(t,n){S=_(function(){t(e.unstable_now())},n)}e.unstable_IdlePriority=5,e.unstable_ImmediatePriority=1,e.unstable_LowPriority=4,e.unstable_NormalPriority=3,e.unstable_Profiling=null,e.unstable_UserBlockingPriority=2,e.unstable_cancelCallback=function(e){e.callback=null},e.unstable_forceFrameRate=function(e){0>e||125<e?console.error(`forceFrameRate takes a positive int between 0 and 125, forcing frame rates higher than 125 fps is not supported`):C=0<e?Math.floor(1e3/e):5},e.unstable_getCurrentPriorityLevel=function(){return f},e.unstable_next=function(e){switch(f){case 1:case 2:case 3:var t=3;break;default:t=f}var n=f;f=t;try{return e()}finally{f=n}},e.unstable_requestPaint=function(){g=!0},e.unstable_runWithPriority=function(e,t){switch(e){case 1:case 2:case 3:case 4:case 5:break;default:e=3}var n=f;f=e;try{return t()}finally{f=n}},e.unstable_scheduleCallback=function(r,i,a){var o=e.unstable_now();switch(typeof a==`object`&&a?(a=a.delay,a=typeof a==`number`&&0<a?o+a:o):a=o,r){case 1:var s=-1;break;case 2:s=250;break;case 5:s=1073741823;break;case 4:s=1e4;break;default:s=5e3}return s=a+s,r={id:u++,callback:i,priorityLevel:r,startTime:a,expirationTime:s,sortIndex:-1},a>o?(r.sortIndex=a,t(l,r),n(c)===null&&r===n(l)&&(h?(v(S),S=-1):h=!0,T(x,a-o))):(r.sortIndex=s,t(c,r),m||p||(m=!0,ee||(ee=!0,re()))),r},e.unstable_shouldYield=ne,e.unstable_wrapCallback=function(e){var t=f;return function(){var n=f;f=t;try{return e.apply(this,arguments)}finally{f=n}}}})),i=e(((e,t)=>{t.exports=r()})),a=e((e=>{var t=n();function r(e){var t=`https://react.dev/errors/`+e;if(1<arguments.length){t+=`?args[]=`+encodeURIComponent(arguments[1]);for(var n=2;n<arguments.length;n++)t+=`&args[]=`+encodeURIComponent(arguments[n])}return`Minified React error #`+e+`; visit `+t+` for the full message or use the non-minified dev environment for full errors and additional helpful warnings.`}function i(){}var a={d:{f:i,r:function(){throw Error(r(522))},D:i,C:i,L:i,m:i,X:i,S:i,M:i},p:0,findDOMNode:null},o=Symbol.for(`react.portal`);function s(e,t,n){var r=3<arguments.length&&arguments[3]!==void 0?arguments[3]:null;return{$$typeof:o,key:r==null?null:``+r,children:e,containerInfo:t,implementation:n}}var c=t.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;function l(e,t){if(e===`font`)return``;if(typeof t==`string`)return t===`use-credentials`?t:``}e.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=a,e.createPortal=function(e,t){var n=2<arguments.length&&arguments[2]!==void 0?arguments[2]:null;if(!t||t.nodeType!==1&&t.nodeType!==9&&t.nodeType!==11)throw Error(r(299));return s(e,t,null,n)},e.flushSync=function(e){var t=c.T,n=a.p;try{if(c.T=null,a.p=2,e)return e()}finally{c.T=t,a.p=n,a.d.f()}},e.preconnect=function(e,t){typeof e==`string`&&(t?(t=t.crossOrigin,t=typeof t==`string`?t===`use-credentials`?t:``:void 0):t=null,a.d.C(e,t))},e.prefetchDNS=function(e){typeof e==`string`&&a.d.D(e)},e.preinit=function(e,t){if(typeof e==`string`&&t&&typeof t.as==`string`){var n=t.as,r=l(n,t.crossOrigin),i=typeof t.integrity==`string`?t.integrity:void 0,o=typeof t.fetchPriority==`string`?t.fetchPriority:void 0;n===`style`?a.d.S(e,typeof t.precedence==`string`?t.precedence:void 0,{crossOrigin:r,integrity:i,fetchPriority:o}):n===`script`&&a.d.X(e,{crossOrigin:r,integrity:i,fetchPriority:o,nonce:typeof t.nonce==`string`?t.nonce:void 0})}},e.preinitModule=function(e,t){if(typeof e==`string`)if(typeof t==`object`&&t){if(t.as==null||t.as===`script`){var n=l(t.as,t.crossOrigin);a.d.M(e,{crossOrigin:n,integrity:typeof t.integrity==`string`?t.integrity:void 0,nonce:typeof t.nonce==`string`?t.nonce:void 0})}}else t??a.d.M(e)},e.preload=function(e,t){if(typeof e==`string`&&typeof t==`object`&&t&&typeof t.as==`string`){var n=t.as,r=l(n,t.crossOrigin);a.d.L(e,n,{crossOrigin:r,integrity:typeof t.integrity==`string`?t.integrity:void 0,nonce:typeof t.nonce==`string`?t.nonce:void 0,type:typeof t.type==`string`?t.type:void 0,fetchPriority:typeof t.fetchPriority==`string`?t.fetchPriority:void 0,referrerPolicy:typeof t.referrerPolicy==`string`?t.referrerPolicy:void 0,imageSrcSet:typeof t.imageSrcSet==`string`?t.imageSrcSet:void 0,imageSizes:typeof t.imageSizes==`string`?t.imageSizes:void 0,media:typeof t.media==`string`?t.media:void 0})}},e.preloadModule=function(e,t){if(typeof e==`string`)if(t){var n=l(t.as,t.crossOrigin);a.d.m(e,{as:typeof t.as==`string`&&t.as!==`script`?t.as:void 0,crossOrigin:n,integrity:typeof t.integrity==`string`?t.integrity:void 0})}else a.d.m(e)},e.requestFormReset=function(e){a.d.r(e)},e.unstable_batchedUpdates=function(e,t){return e(t)},e.useFormState=function(e,t,n){return c.H.useFormState(e,t,n)},e.useFormStatus=function(){return c.H.useHostTransitionStatus()},e.version=`19.2.7`})),o=e(((e,t)=>{function n(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>`u`||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!=`function`))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(n)}catch(e){console.error(e)}}n(),t.exports=a()})),s=e((e=>{var t=i(),r=n(),a=o();function s(e){var t=`https://react.dev/errors/`+e;if(1<arguments.length){t+=`?args[]=`+encodeURIComponent(arguments[1]);for(var n=2;n<arguments.length;n++)t+=`&args[]=`+encodeURIComponent(arguments[n])}return`Minified React error #`+e+`; visit `+t+` for the full message or use the non-minified dev environment for full errors and additional helpful warnings.`}function c(e){return!(!e||e.nodeType!==1&&e.nodeType!==9&&e.nodeType!==11)}function l(e){var t=e,n=e;if(e.alternate)for(;t.return;)t=t.return;else{e=t;do t=e,t.flags&4098&&(n=t.return),e=t.return;while(e)}return t.tag===3?n:null}function u(e){if(e.tag===13){var t=e.memoizedState;if(t===null&&(e=e.alternate,e!==null&&(t=e.memoizedState)),t!==null)return t.dehydrated}return null}function d(e){if(e.tag===31){var t=e.memoizedState;if(t===null&&(e=e.alternate,e!==null&&(t=e.memoizedState)),t!==null)return t.dehydrated}return null}function f(e){if(l(e)!==e)throw Error(s(188))}function p(e){var t=e.alternate;if(!t){if(t=l(e),t===null)throw Error(s(188));return t===e?e:null}for(var n=e,r=t;;){var i=n.return;if(i===null)break;var a=i.alternate;if(a===null){if(r=i.return,r!==null){n=r;continue}break}if(i.child===a.child){for(a=i.child;a;){if(a===n)return f(i),e;if(a===r)return f(i),t;a=a.sibling}throw Error(s(188))}if(n.return!==r.return)n=i,r=a;else{for(var o=!1,c=i.child;c;){if(c===n){o=!0,n=i,r=a;break}if(c===r){o=!0,r=i,n=a;break}c=c.sibling}if(!o){for(c=a.child;c;){if(c===n){o=!0,n=a,r=i;break}if(c===r){o=!0,r=a,n=i;break}c=c.sibling}if(!o)throw Error(s(189))}}if(n.alternate!==r)throw Error(s(190))}if(n.tag!==3)throw Error(s(188));return n.stateNode.current===n?e:t}function m(e){var t=e.tag;if(t===5||t===26||t===27||t===6)return e;for(e=e.child;e!==null;){if(t=m(e),t!==null)return t;e=e.sibling}return null}var h=Object.assign,g=Symbol.for(`react.element`),_=Symbol.for(`react.transitional.element`),v=Symbol.for(`react.portal`),y=Symbol.for(`react.fragment`),b=Symbol.for(`react.strict_mode`),x=Symbol.for(`react.profiler`),ee=Symbol.for(`react.consumer`),S=Symbol.for(`react.context`),C=Symbol.for(`react.forward_ref`),te=Symbol.for(`react.suspense`),ne=Symbol.for(`react.suspense_list`),w=Symbol.for(`react.memo`),re=Symbol.for(`react.lazy`),ie=Symbol.for(`react.activity`),ae=Symbol.for(`react.memo_cache_sentinel`),T=Symbol.iterator;function oe(e){return typeof e!=`object`||!e?null:(e=T&&e[T]||e[`@@iterator`],typeof e==`function`?e:null)}var se=Symbol.for(`react.client.reference`);function ce(e){if(e==null)return null;if(typeof e==`function`)return e.$$typeof===se?null:e.displayName||e.name||null;if(typeof e==`string`)return e;switch(e){case y:return`Fragment`;case x:return`Profiler`;case b:return`StrictMode`;case te:return`Suspense`;case ne:return`SuspenseList`;case ie:return`Activity`}if(typeof e==`object`)switch(e.$$typeof){case v:return`Portal`;case S:return e.displayName||`Context`;case ee:return(e._context.displayName||`Context`)+`.Consumer`;case C:var t=e.render;return e=e.displayName,e||=(e=t.displayName||t.name||``,e===``?`ForwardRef`:`ForwardRef(`+e+`)`),e;case w:return t=e.displayName||null,t===null?ce(e.type)||`Memo`:t;case re:t=e._payload,e=e._init;try{return ce(e(t))}catch{}}return null}var le=Array.isArray,E=r.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,D=a.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,ue={pending:!1,data:null,method:null,action:null},de=[],fe=-1;function pe(e){return{current:e}}function me(e){0>fe||(e.current=de[fe],de[fe]=null,fe--)}function he(e,t){fe++,de[fe]=e.current,e.current=t}var ge=pe(null),_e=pe(null),ve=pe(null),ye=pe(null);function be(e,t){switch(he(ve,t),he(_e,e),he(ge,null),t.nodeType){case 9:case 11:e=(e=t.documentElement)&&(e=e.namespaceURI)?Gd(e):0;break;default:if(e=t.tagName,t=t.namespaceURI)t=Gd(t),e=Kd(t,e);else switch(e){case`svg`:e=1;break;case`math`:e=2;break;default:e=0}}me(ge),he(ge,e)}function xe(){me(ge),me(_e),me(ve)}function Se(e){e.memoizedState!==null&&he(ye,e);var t=ge.current,n=Kd(t,e.type);t!==n&&(he(_e,e),he(ge,n))}function Ce(e){_e.current===e&&(me(ge),me(_e)),ye.current===e&&(me(ye),tp._currentValue=ue)}var we,Te;function Ee(e){if(we===void 0)try{throw Error()}catch(e){var t=e.stack.trim().match(/\n( *(at )?)/);we=t&&t[1]||``,Te=-1<e.stack.indexOf(`
    at`)?` (<anonymous>)`:-1<e.stack.indexOf(`@`)?`@unknown:0:0`:``}return`
`+we+e+Te}var De=!1;function Oe(e,t){if(!e||De)return``;De=!0;var n=Error.prepareStackTrace;Error.prepareStackTrace=void 0;try{var r={DetermineComponentFrameRoot:function(){try{if(t){var n=function(){throw Error()};if(Object.defineProperty(n.prototype,"props",{set:function(){throw Error()}}),typeof Reflect==`object`&&Reflect.construct){try{Reflect.construct(n,[])}catch(e){var r=e}Reflect.construct(e,[],n)}else{try{n.call()}catch(e){r=e}e.call(n.prototype)}}else{try{throw Error()}catch(e){r=e}(n=e())&&typeof n.catch==`function`&&n.catch(function(){})}}catch(e){if(e&&r&&typeof e.stack==`string`)return[e.stack,r.stack]}return[null,null]}};r.DetermineComponentFrameRoot.displayName=`DetermineComponentFrameRoot`;var i=Object.getOwnPropertyDescriptor(r.DetermineComponentFrameRoot,`name`);i&&i.configurable&&Object.defineProperty(r.DetermineComponentFrameRoot,"name",{value:`DetermineComponentFrameRoot`});var a=r.DetermineComponentFrameRoot(),o=a[0],s=a[1];if(o&&s){var c=o.split(`
`),l=s.split(`
`);for(i=r=0;r<c.length&&!c[r].includes(`DetermineComponentFrameRoot`);)r++;for(;i<l.length&&!l[i].includes(`DetermineComponentFrameRoot`);)i++;if(r===c.length||i===l.length)for(r=c.length-1,i=l.length-1;1<=r&&0<=i&&c[r]!==l[i];)i--;for(;1<=r&&0<=i;r--,i--)if(c[r]!==l[i]){if(r!==1||i!==1)do if(r--,i--,0>i||c[r]!==l[i]){var u=`
`+c[r].replace(` at new `,` at `);return e.displayName&&u.includes(`<anonymous>`)&&(u=u.replace(`<anonymous>`,e.displayName)),u}while(1<=r&&0<=i);break}}}finally{De=!1,Error.prepareStackTrace=n}return(n=e?e.displayName||e.name:``)?Ee(n):``}function ke(e,t){switch(e.tag){case 26:case 27:case 5:return Ee(e.type);case 16:return Ee(`Lazy`);case 13:return e.child!==t&&t!==null?Ee(`Suspense Fallback`):Ee(`Suspense`);case 19:return Ee(`SuspenseList`);case 0:case 15:return Oe(e.type,!1);case 11:return Oe(e.type.render,!1);case 1:return Oe(e.type,!0);case 31:return Ee(`Activity`);default:return``}}function Ae(e){try{var t=``,n=null;do t+=ke(e,n),n=e,e=e.return;while(e);return t}catch(e){return`
Error generating stack: `+e.message+`
`+e.stack}}var je=Object.prototype.hasOwnProperty,Me=t.unstable_scheduleCallback,Ne=t.unstable_cancelCallback,Pe=t.unstable_shouldYield,Fe=t.unstable_requestPaint,Ie=t.unstable_now,Le=t.unstable_getCurrentPriorityLevel,Re=t.unstable_ImmediatePriority,ze=t.unstable_UserBlockingPriority,Be=t.unstable_NormalPriority,Ve=t.unstable_LowPriority,He=t.unstable_IdlePriority,Ue=t.log,We=t.unstable_setDisableYieldValue,Ge=null,Ke=null;function qe(e){if(typeof Ue==`function`&&We(e),Ke&&typeof Ke.setStrictMode==`function`)try{Ke.setStrictMode(Ge,e)}catch{}}var Je=Math.clz32?Math.clz32:Ze,Ye=Math.log,Xe=Math.LN2;function Ze(e){return e>>>=0,e===0?32:31-(Ye(e)/Xe|0)|0}var Qe=256,$e=262144,et=4194304;function tt(e){var t=e&42;if(t!==0)return t;switch(e&-e){case 1:return 1;case 2:return 2;case 4:return 4;case 8:return 8;case 16:return 16;case 32:return 32;case 64:return 64;case 128:return 128;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:return e&261888;case 262144:case 524288:case 1048576:case 2097152:return e&3932160;case 4194304:case 8388608:case 16777216:case 33554432:return e&62914560;case 67108864:return 67108864;case 134217728:return 134217728;case 268435456:return 268435456;case 536870912:return 536870912;case 1073741824:return 0;default:return e}}function nt(e,t,n){var r=e.pendingLanes;if(r===0)return 0;var i=0,a=e.suspendedLanes,o=e.pingedLanes;e=e.warmLanes;var s=r&134217727;return s===0?(s=r&~a,s===0?o===0?n||(n=r&~e,n!==0&&(i=tt(n))):i=tt(o):i=tt(s)):(r=s&~a,r===0?(o&=s,o===0?n||(n=s&~e,n!==0&&(i=tt(n))):i=tt(o)):i=tt(r)),i===0?0:t!==0&&t!==i&&(t&a)===0&&(a=i&-i,n=t&-t,a>=n||a===32&&n&4194048)?t:i}function rt(e,t){return(e.pendingLanes&~(e.suspendedLanes&~e.pingedLanes)&t)===0}function it(e,t){switch(e){case 1:case 2:case 4:case 8:case 64:return t+250;case 16:case 32:case 128:case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:return t+5e3;case 4194304:case 8388608:case 16777216:case 33554432:return-1;case 67108864:case 134217728:case 268435456:case 536870912:case 1073741824:return-1;default:return-1}}function at(){var e=et;return et<<=1,!(et&62914560)&&(et=4194304),e}function ot(e){for(var t=[],n=0;31>n;n++)t.push(e);return t}function st(e,t){e.pendingLanes|=t,t!==268435456&&(e.suspendedLanes=0,e.pingedLanes=0,e.warmLanes=0)}function ct(e,t,n,r,i,a){var o=e.pendingLanes;e.pendingLanes=n,e.suspendedLanes=0,e.pingedLanes=0,e.warmLanes=0,e.expiredLanes&=n,e.entangledLanes&=n,e.errorRecoveryDisabledLanes&=n,e.shellSuspendCounter=0;var s=e.entanglements,c=e.expirationTimes,l=e.hiddenUpdates;for(n=o&~n;0<n;){var u=31-Je(n),d=1<<u;s[u]=0,c[u]=-1;var f=l[u];if(f!==null)for(l[u]=null,u=0;u<f.length;u++){var p=f[u];p!==null&&(p.lane&=-536870913)}n&=~d}r!==0&&lt(e,r,0),a!==0&&i===0&&e.tag!==0&&(e.suspendedLanes|=a&~(o&~t))}function lt(e,t,n){e.pendingLanes|=t,e.suspendedLanes&=~t;var r=31-Je(t);e.entangledLanes|=t,e.entanglements[r]=e.entanglements[r]|1073741824|n&261930}function ut(e,t){var n=e.entangledLanes|=t;for(e=e.entanglements;n;){var r=31-Je(n),i=1<<r;i&t|e[r]&t&&(e[r]|=t),n&=~i}}function dt(e,t){var n=t&-t;return n=n&42?1:ft(n),(n&(e.suspendedLanes|t))===0?n:0}function ft(e){switch(e){case 2:e=1;break;case 8:e=4;break;case 32:e=16;break;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:case 4194304:case 8388608:case 16777216:case 33554432:e=128;break;case 268435456:e=134217728;break;default:e=0}return e}function pt(e){return e&=-e,2<e?8<e?e&134217727?32:268435456:8:2}function mt(){var e=D.p;return e===0?(e=window.event,e===void 0?32:_p(e.type)):e}function ht(e,t){var n=D.p;try{return D.p=e,t()}finally{D.p=n}}var gt=Math.random().toString(36).slice(2),_t=`__reactFiber$`+gt,vt=`__reactProps$`+gt,yt=`__reactContainer$`+gt,bt=`__reactEvents$`+gt,xt=`__reactListeners$`+gt,St=`__reactHandles$`+gt,Ct=`__reactResources$`+gt,wt=`__reactMarker$`+gt;function Tt(e){delete e[_t],delete e[vt],delete e[bt],delete e[xt],delete e[St]}function Et(e){var t=e[_t];if(t)return t;for(var n=e.parentNode;n;){if(t=n[yt]||n[_t]){if(n=t.alternate,t.child!==null||n!==null&&n.child!==null)for(e=mf(e);e!==null;){if(n=e[_t])return n;e=mf(e)}return t}e=n,n=e.parentNode}return null}function Dt(e){if(e=e[_t]||e[yt]){var t=e.tag;if(t===5||t===6||t===13||t===31||t===26||t===27||t===3)return e}return null}function Ot(e){var t=e.tag;if(t===5||t===26||t===27||t===6)return e.stateNode;throw Error(s(33))}function kt(e){var t=e[Ct];return t||=e[Ct]={hoistableStyles:new Map,hoistableScripts:new Map},t}function At(e){e[wt]=!0}var jt=new Set,Mt={};function Nt(e,t){Pt(e,t),Pt(e+`Capture`,t)}function Pt(e,t){for(Mt[e]=t,e=0;e<t.length;e++)jt.add(t[e])}var Ft=RegExp(`^[:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD][:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD\\-.0-9\\u00B7\\u0300-\\u036F\\u203F-\\u2040]*$`),It={},Lt={};function Rt(e){return je.call(Lt,e)?!0:je.call(It,e)?!1:Ft.test(e)?Lt[e]=!0:(It[e]=!0,!1)}function zt(e,t,n){if(Rt(t))if(n===null)e.removeAttribute(t);else{switch(typeof n){case`undefined`:case`function`:case`symbol`:e.removeAttribute(t);return;case`boolean`:var r=t.toLowerCase().slice(0,5);if(r!==`data-`&&r!==`aria-`){e.removeAttribute(t);return}}e.setAttribute(t,``+n)}}function Bt(e,t,n){if(n===null)e.removeAttribute(t);else{switch(typeof n){case`undefined`:case`function`:case`symbol`:case`boolean`:e.removeAttribute(t);return}e.setAttribute(t,``+n)}}function Vt(e,t,n,r){if(r===null)e.removeAttribute(n);else{switch(typeof r){case`undefined`:case`function`:case`symbol`:case`boolean`:e.removeAttribute(n);return}e.setAttributeNS(t,n,``+r)}}function Ht(e){switch(typeof e){case`bigint`:case`boolean`:case`number`:case`string`:case`undefined`:return e;case`object`:return e;default:return``}}function Ut(e){var t=e.type;return(e=e.nodeName)&&e.toLowerCase()===`input`&&(t===`checkbox`||t===`radio`)}function Wt(e,t,n){var r=Object.getOwnPropertyDescriptor(e.constructor.prototype,t);if(!e.hasOwnProperty(t)&&r!==void 0&&typeof r.get==`function`&&typeof r.set==`function`){var i=r.get,a=r.set;return Object.defineProperty(e,t,{configurable:!0,get:function(){return i.call(this)},set:function(e){n=``+e,a.call(this,e)}}),Object.defineProperty(e,t,{enumerable:r.enumerable}),{getValue:function(){return n},setValue:function(e){n=``+e},stopTracking:function(){e._valueTracker=null,delete e[t]}}}}function Gt(e){if(!e._valueTracker){var t=Ut(e)?`checked`:`value`;e._valueTracker=Wt(e,t,``+e[t])}}function Kt(e){if(!e)return!1;var t=e._valueTracker;if(!t)return!0;var n=t.getValue(),r=``;return e&&(r=Ut(e)?e.checked?`true`:`false`:e.value),e=r,e===n?!1:(t.setValue(e),!0)}function qt(e){if(e||=typeof document<`u`?document:void 0,e===void 0)return null;try{return e.activeElement||e.body}catch{return e.body}}var Jt=/[\n"\\]/g;function Yt(e){return e.replace(Jt,function(e){return`\\`+e.charCodeAt(0).toString(16)+` `})}function Xt(e,t,n,r,i,a,o,s){e.name=``,o!=null&&typeof o!=`function`&&typeof o!=`symbol`&&typeof o!=`boolean`?e.type=o:e.removeAttribute(`type`),t==null?o!==`submit`&&o!==`reset`||e.removeAttribute(`value`):o===`number`?(t===0&&e.value===``||e.value!=t)&&(e.value=``+Ht(t)):e.value!==``+Ht(t)&&(e.value=``+Ht(t)),t==null?n==null?r!=null&&e.removeAttribute(`value`):Qt(e,o,Ht(n)):Qt(e,o,Ht(t)),i==null&&a!=null&&(e.defaultChecked=!!a),i!=null&&(e.checked=i&&typeof i!=`function`&&typeof i!=`symbol`),s!=null&&typeof s!=`function`&&typeof s!=`symbol`&&typeof s!=`boolean`?e.name=``+Ht(s):e.removeAttribute(`name`)}function Zt(e,t,n,r,i,a,o,s){if(a!=null&&typeof a!=`function`&&typeof a!=`symbol`&&typeof a!=`boolean`&&(e.type=a),t!=null||n!=null){if(!(a!==`submit`&&a!==`reset`||t!=null)){Gt(e);return}n=n==null?``:``+Ht(n),t=t==null?n:``+Ht(t),s||t===e.value||(e.value=t),e.defaultValue=t}r??=i,r=typeof r!=`function`&&typeof r!=`symbol`&&!!r,e.checked=s?e.checked:!!r,e.defaultChecked=!!r,o!=null&&typeof o!=`function`&&typeof o!=`symbol`&&typeof o!=`boolean`&&(e.name=o),Gt(e)}function Qt(e,t,n){t===`number`&&qt(e.ownerDocument)===e||e.defaultValue===``+n||(e.defaultValue=``+n)}function $t(e,t,n,r){if(e=e.options,t){t={};for(var i=0;i<n.length;i++)t[`$`+n[i]]=!0;for(n=0;n<e.length;n++)i=t.hasOwnProperty(`$`+e[n].value),e[n].selected!==i&&(e[n].selected=i),i&&r&&(e[n].defaultSelected=!0)}else{for(n=``+Ht(n),t=null,i=0;i<e.length;i++){if(e[i].value===n){e[i].selected=!0,r&&(e[i].defaultSelected=!0);return}t!==null||e[i].disabled||(t=e[i])}t!==null&&(t.selected=!0)}}function en(e,t,n){if(t!=null&&(t=``+Ht(t),t!==e.value&&(e.value=t),n==null)){e.defaultValue!==t&&(e.defaultValue=t);return}e.defaultValue=n==null?``:``+Ht(n)}function tn(e,t,n,r){if(t==null){if(r!=null){if(n!=null)throw Error(s(92));if(le(r)){if(1<r.length)throw Error(s(93));r=r[0]}n=r}n??=``,t=n}n=Ht(t),e.defaultValue=n,r=e.textContent,r===n&&r!==``&&r!==null&&(e.value=r),Gt(e)}function O(e,t){if(t){var n=e.firstChild;if(n&&n===e.lastChild&&n.nodeType===3){n.nodeValue=t;return}}e.textContent=t}var nn=new Set(`animationIterationCount aspectRatio borderImageOutset borderImageSlice borderImageWidth boxFlex boxFlexGroup boxOrdinalGroup columnCount columns flex flexGrow flexPositive flexShrink flexNegative flexOrder gridArea gridRow gridRowEnd gridRowSpan gridRowStart gridColumn gridColumnEnd gridColumnSpan gridColumnStart fontWeight lineClamp lineHeight opacity order orphans scale tabSize widows zIndex zoom fillOpacity floodOpacity stopOpacity strokeDasharray strokeDashoffset strokeMiterlimit strokeOpacity strokeWidth MozAnimationIterationCount MozBoxFlex MozBoxFlexGroup MozLineClamp msAnimationIterationCount msFlex msZoom msFlexGrow msFlexNegative msFlexOrder msFlexPositive msFlexShrink msGridColumn msGridColumnSpan msGridRow msGridRowSpan WebkitAnimationIterationCount WebkitBoxFlex WebKitBoxFlexGroup WebkitBoxOrdinalGroup WebkitColumnCount WebkitColumns WebkitFlex WebkitFlexGrow WebkitFlexPositive WebkitFlexShrink WebkitLineClamp`.split(` `));function rn(e,t,n){var r=t.indexOf(`--`)===0;n==null||typeof n==`boolean`||n===``?r?e.setProperty(t,``):t===`float`?e.cssFloat=``:e[t]=``:r?e.setProperty(t,n):typeof n!=`number`||n===0||nn.has(t)?t===`float`?e.cssFloat=n:e[t]=(``+n).trim():e[t]=n+`px`}function an(e,t,n){if(t!=null&&typeof t!=`object`)throw Error(s(62));if(e=e.style,n!=null){for(var r in n)!n.hasOwnProperty(r)||t!=null&&t.hasOwnProperty(r)||(r.indexOf(`--`)===0?e.setProperty(r,``):r===`float`?e.cssFloat=``:e[r]=``);for(var i in t)r=t[i],t.hasOwnProperty(i)&&n[i]!==r&&rn(e,i,r)}else for(var a in t)t.hasOwnProperty(a)&&rn(e,a,t[a])}function on(e){if(e.indexOf(`-`)===-1)return!1;switch(e){case`annotation-xml`:case`color-profile`:case`font-face`:case`font-face-src`:case`font-face-uri`:case`font-face-format`:case`font-face-name`:case`missing-glyph`:return!1;default:return!0}}var sn=new Map([[`acceptCharset`,`accept-charset`],[`htmlFor`,`for`],[`httpEquiv`,`http-equiv`],[`crossOrigin`,`crossorigin`],[`accentHeight`,`accent-height`],[`alignmentBaseline`,`alignment-baseline`],[`arabicForm`,`arabic-form`],[`baselineShift`,`baseline-shift`],[`capHeight`,`cap-height`],[`clipPath`,`clip-path`],[`clipRule`,`clip-rule`],[`colorInterpolation`,`color-interpolation`],[`colorInterpolationFilters`,`color-interpolation-filters`],[`colorProfile`,`color-profile`],[`colorRendering`,`color-rendering`],[`dominantBaseline`,`dominant-baseline`],[`enableBackground`,`enable-background`],[`fillOpacity`,`fill-opacity`],[`fillRule`,`fill-rule`],[`floodColor`,`flood-color`],[`floodOpacity`,`flood-opacity`],[`fontFamily`,`font-family`],[`fontSize`,`font-size`],[`fontSizeAdjust`,`font-size-adjust`],[`fontStretch`,`font-stretch`],[`fontStyle`,`font-style`],[`fontVariant`,`font-variant`],[`fontWeight`,`font-weight`],[`glyphName`,`glyph-name`],[`glyphOrientationHorizontal`,`glyph-orientation-horizontal`],[`glyphOrientationVertical`,`glyph-orientation-vertical`],[`horizAdvX`,`horiz-adv-x`],[`horizOriginX`,`horiz-origin-x`],[`imageRendering`,`image-rendering`],[`letterSpacing`,`letter-spacing`],[`lightingColor`,`lighting-color`],[`markerEnd`,`marker-end`],[`markerMid`,`marker-mid`],[`markerStart`,`marker-start`],[`overlinePosition`,`overline-position`],[`overlineThickness`,`overline-thickness`],[`paintOrder`,`paint-order`],[`panose-1`,`panose-1`],[`pointerEvents`,`pointer-events`],[`renderingIntent`,`rendering-intent`],[`shapeRendering`,`shape-rendering`],[`stopColor`,`stop-color`],[`stopOpacity`,`stop-opacity`],[`strikethroughPosition`,`strikethrough-position`],[`strikethroughThickness`,`strikethrough-thickness`],[`strokeDasharray`,`stroke-dasharray`],[`strokeDashoffset`,`stroke-dashoffset`],[`strokeLinecap`,`stroke-linecap`],[`strokeLinejoin`,`stroke-linejoin`],[`strokeMiterlimit`,`stroke-miterlimit`],[`strokeOpacity`,`stroke-opacity`],[`strokeWidth`,`stroke-width`],[`textAnchor`,`text-anchor`],[`textDecoration`,`text-decoration`],[`textRendering`,`text-rendering`],[`transformOrigin`,`transform-origin`],[`underlinePosition`,`underline-position`],[`underlineThickness`,`underline-thickness`],[`unicodeBidi`,`unicode-bidi`],[`unicodeRange`,`unicode-range`],[`unitsPerEm`,`units-per-em`],[`vAlphabetic`,`v-alphabetic`],[`vHanging`,`v-hanging`],[`vIdeographic`,`v-ideographic`],[`vMathematical`,`v-mathematical`],[`vectorEffect`,`vector-effect`],[`vertAdvY`,`vert-adv-y`],[`vertOriginX`,`vert-origin-x`],[`vertOriginY`,`vert-origin-y`],[`wordSpacing`,`word-spacing`],[`writingMode`,`writing-mode`],[`xmlnsXlink`,`xmlns:xlink`],[`xHeight`,`x-height`]]),cn=/^[\u0000-\u001F ]*j[\r\n\t]*a[\r\n\t]*v[\r\n\t]*a[\r\n\t]*s[\r\n\t]*c[\r\n\t]*r[\r\n\t]*i[\r\n\t]*p[\r\n\t]*t[\r\n\t]*:/i;function ln(e){return cn.test(``+e)?`javascript:throw new Error('React has blocked a javascript: URL as a security precaution.')`:e}function un(){}var dn=null;function fn(e){return e=e.target||e.srcElement||window,e.correspondingUseElement&&(e=e.correspondingUseElement),e.nodeType===3?e.parentNode:e}var pn=null,mn=null;function hn(e){var t=Dt(e);if(t&&(e=t.stateNode)){var n=e[vt]||null;a:switch(e=t.stateNode,t.type){case`input`:if(Xt(e,n.value,n.defaultValue,n.defaultValue,n.checked,n.defaultChecked,n.type,n.name),t=n.name,n.type===`radio`&&t!=null){for(n=e;n.parentNode;)n=n.parentNode;for(n=n.querySelectorAll(`input[name="`+Yt(``+t)+`"][type="radio"]`),t=0;t<n.length;t++){var r=n[t];if(r!==e&&r.form===e.form){var i=r[vt]||null;if(!i)throw Error(s(90));Xt(r,i.value,i.defaultValue,i.defaultValue,i.checked,i.defaultChecked,i.type,i.name)}}for(t=0;t<n.length;t++)r=n[t],r.form===e.form&&Kt(r)}break a;case`textarea`:en(e,n.value,n.defaultValue);break a;case`select`:t=n.value,t!=null&&$t(e,!!n.multiple,t,!1)}}}var gn=!1;function _n(e,t,n){if(gn)return e(t,n);gn=!0;try{return e(t)}finally{if(gn=!1,(pn!==null||mn!==null)&&(Cu(),pn&&(t=pn,e=mn,mn=pn=null,hn(t),e)))for(t=0;t<e.length;t++)hn(e[t])}}function vn(e,t){var n=e.stateNode;if(n===null)return null;var r=n[vt]||null;if(r===null)return null;n=r[t];a:switch(t){case`onClick`:case`onClickCapture`:case`onDoubleClick`:case`onDoubleClickCapture`:case`onMouseDown`:case`onMouseDownCapture`:case`onMouseMove`:case`onMouseMoveCapture`:case`onMouseUp`:case`onMouseUpCapture`:case`onMouseEnter`:(r=!r.disabled)||(e=e.type,r=!(e===`button`||e===`input`||e===`select`||e===`textarea`)),e=!r;break a;default:e=!1}if(e)return null;if(n&&typeof n!=`function`)throw Error(s(231,t,typeof n));return n}var yn=!(typeof window>`u`||window.document===void 0||window.document.createElement===void 0),bn=!1;if(yn)try{var xn={};Object.defineProperty(xn,"passive",{get:function(){bn=!0}}),window.addEventListener(`test`,xn,xn),window.removeEventListener(`test`,xn,xn)}catch{bn=!1}var Sn=null,Cn=null,wn=null;function Tn(){if(wn)return wn;var e,t=Cn,n=t.length,r,i=`value`in Sn?Sn.value:Sn.textContent,a=i.length;for(e=0;e<n&&t[e]===i[e];e++);var o=n-e;for(r=1;r<=o&&t[n-r]===i[a-r];r++);return wn=i.slice(e,1<r?1-r:void 0)}function En(e){var t=e.keyCode;return`charCode`in e?(e=e.charCode,e===0&&t===13&&(e=13)):e=t,e===10&&(e=13),32<=e||e===13?e:0}function Dn(){return!0}function On(){return!1}function kn(e){function t(t,n,r,i,a){for(var o in this._reactName=t,this._targetInst=r,this.type=n,this.nativeEvent=i,this.target=a,this.currentTarget=null,e)e.hasOwnProperty(o)&&(t=e[o],this[o]=t?t(i):i[o]);return this.isDefaultPrevented=(i.defaultPrevented==null?!1===i.returnValue:i.defaultPrevented)?Dn:On,this.isPropagationStopped=On,this}return h(t.prototype,{preventDefault:function(){this.defaultPrevented=!0;var e=this.nativeEvent;e&&(e.preventDefault?e.preventDefault():typeof e.returnValue!=`unknown`&&(e.returnValue=!1),this.isDefaultPrevented=Dn)},stopPropagation:function(){var e=this.nativeEvent;e&&(e.stopPropagation?e.stopPropagation():typeof e.cancelBubble!=`unknown`&&(e.cancelBubble=!0),this.isPropagationStopped=Dn)},persist:function(){},isPersistent:Dn}),t}var An={eventPhase:0,bubbles:0,cancelable:0,timeStamp:function(e){return e.timeStamp||Date.now()},defaultPrevented:0,isTrusted:0},jn=kn(An),Mn=h({},An,{view:0,detail:0}),Nn=kn(Mn),Pn,Fn,In,Ln=h({},Mn,{screenX:0,screenY:0,clientX:0,clientY:0,pageX:0,pageY:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,getModifierState:Jn,button:0,buttons:0,relatedTarget:function(e){return e.relatedTarget===void 0?e.fromElement===e.srcElement?e.toElement:e.fromElement:e.relatedTarget},movementX:function(e){return`movementX`in e?e.movementX:(e!==In&&(In&&e.type===`mousemove`?(Pn=e.screenX-In.screenX,Fn=e.screenY-In.screenY):Fn=Pn=0,In=e),Pn)},movementY:function(e){return`movementY`in e?e.movementY:Fn}}),Rn=kn(Ln),zn=kn(h({},Ln,{dataTransfer:0})),Bn=kn(h({},Mn,{relatedTarget:0})),Vn=kn(h({},An,{animationName:0,elapsedTime:0,pseudoElement:0})),Hn=kn(h({},An,{clipboardData:function(e){return`clipboardData`in e?e.clipboardData:window.clipboardData}})),Un=kn(h({},An,{data:0})),Wn={Esc:`Escape`,Spacebar:` `,Left:`ArrowLeft`,Up:`ArrowUp`,Right:`ArrowRight`,Down:`ArrowDown`,Del:`Delete`,Win:`OS`,Menu:`ContextMenu`,Apps:`ContextMenu`,Scroll:`ScrollLock`,MozPrintableKey:`Unidentified`},Gn={8:`Backspace`,9:`Tab`,12:`Clear`,13:`Enter`,16:`Shift`,17:`Control`,18:`Alt`,19:`Pause`,20:`CapsLock`,27:`Escape`,32:` `,33:`PageUp`,34:`PageDown`,35:`End`,36:`Home`,37:`ArrowLeft`,38:`ArrowUp`,39:`ArrowRight`,40:`ArrowDown`,45:`Insert`,46:`Delete`,112:`F1`,113:`F2`,114:`F3`,115:`F4`,116:`F5`,117:`F6`,118:`F7`,119:`F8`,120:`F9`,121:`F10`,122:`F11`,123:`F12`,144:`NumLock`,145:`ScrollLock`,224:`Meta`},Kn={Alt:`altKey`,Control:`ctrlKey`,Meta:`metaKey`,Shift:`shiftKey`};function qn(e){var t=this.nativeEvent;return t.getModifierState?t.getModifierState(e):(e=Kn[e])?!!t[e]:!1}function Jn(){return qn}var Yn=kn(h({},Mn,{key:function(e){if(e.key){var t=Wn[e.key]||e.key;if(t!==`Unidentified`)return t}return e.type===`keypress`?(e=En(e),e===13?`Enter`:String.fromCharCode(e)):e.type===`keydown`||e.type===`keyup`?Gn[e.keyCode]||`Unidentified`:``},code:0,location:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,repeat:0,locale:0,getModifierState:Jn,charCode:function(e){return e.type===`keypress`?En(e):0},keyCode:function(e){return e.type===`keydown`||e.type===`keyup`?e.keyCode:0},which:function(e){return e.type===`keypress`?En(e):e.type===`keydown`||e.type===`keyup`?e.keyCode:0}})),Xn=kn(h({},Ln,{pointerId:0,width:0,height:0,pressure:0,tangentialPressure:0,tiltX:0,tiltY:0,twist:0,pointerType:0,isPrimary:0})),Zn=kn(h({},Mn,{touches:0,targetTouches:0,changedTouches:0,altKey:0,metaKey:0,ctrlKey:0,shiftKey:0,getModifierState:Jn})),Qn=kn(h({},An,{propertyName:0,elapsedTime:0,pseudoElement:0})),$n=kn(h({},Ln,{deltaX:function(e){return`deltaX`in e?e.deltaX:`wheelDeltaX`in e?-e.wheelDeltaX:0},deltaY:function(e){return`deltaY`in e?e.deltaY:`wheelDeltaY`in e?-e.wheelDeltaY:`wheelDelta`in e?-e.wheelDelta:0},deltaZ:0,deltaMode:0})),er=kn(h({},An,{newState:0,oldState:0})),tr=[9,13,27,32],nr=yn&&`CompositionEvent`in window,rr=null;yn&&`documentMode`in document&&(rr=document.documentMode);var ir=yn&&`TextEvent`in window&&!rr,ar=yn&&(!nr||rr&&8<rr&&11>=rr),or=` `,sr=!1;function cr(e,t){switch(e){case`keyup`:return tr.indexOf(t.keyCode)!==-1;case`keydown`:return t.keyCode!==229;case`keypress`:case`mousedown`:case`focusout`:return!0;default:return!1}}function lr(e){return e=e.detail,typeof e==`object`&&`data`in e?e.data:null}var ur=!1;function dr(e,t){switch(e){case`compositionend`:return lr(t);case`keypress`:return t.which===32?(sr=!0,or):null;case`textInput`:return e=t.data,e===or&&sr?null:e;default:return null}}function fr(e,t){if(ur)return e===`compositionend`||!nr&&cr(e,t)?(e=Tn(),wn=Cn=Sn=null,ur=!1,e):null;switch(e){case`paste`:return null;case`keypress`:if(!(t.ctrlKey||t.altKey||t.metaKey)||t.ctrlKey&&t.altKey){if(t.char&&1<t.char.length)return t.char;if(t.which)return String.fromCharCode(t.which)}return null;case`compositionend`:return ar&&t.locale!==`ko`?null:t.data;default:return null}}var pr={color:!0,date:!0,datetime:!0,"datetime-local":!0,email:!0,month:!0,number:!0,password:!0,range:!0,search:!0,tel:!0,text:!0,time:!0,url:!0,week:!0};function mr(e){var t=e&&e.nodeName&&e.nodeName.toLowerCase();return t===`input`?!!pr[e.type]:t===`textarea`}function hr(e,t,n,r){pn?mn?mn.push(r):mn=[r]:pn=r,t=kd(t,`onChange`),0<t.length&&(n=new jn(`onChange`,`change`,null,n,r),e.push({event:n,listeners:t}))}var gr=null,_r=null;function vr(e){Sd(e,0)}function yr(e){if(Kt(Ot(e)))return e}function br(e,t){if(e===`change`)return t}var xr=!1;if(yn){var Sr;if(yn){var Cr=`oninput`in document;if(!Cr){var k=document.createElement(`div`);k.setAttribute(`oninput`,`return;`),Cr=typeof k.oninput==`function`}Sr=Cr}else Sr=!1;xr=Sr&&(!document.documentMode||9<document.documentMode)}function wr(){gr&&(gr.detachEvent(`onpropertychange`,Tr),_r=gr=null)}function Tr(e){if(e.propertyName===`value`&&yr(_r)){var t=[];hr(t,_r,e,fn(e)),_n(vr,t)}}function Er(e,t,n){e===`focusin`?(wr(),gr=t,_r=n,gr.attachEvent(`onpropertychange`,Tr)):e===`focusout`&&wr()}function Dr(e){if(e===`selectionchange`||e===`keyup`||e===`keydown`)return yr(_r)}function Or(e,t){if(e===`click`)return yr(t)}function kr(e,t){if(e===`input`||e===`change`)return yr(t)}function Ar(e,t){return e===t&&(e!==0||1/e==1/t)||e!==e&&t!==t}var jr=typeof Object.is==`function`?Object.is:Ar;function Mr(e,t){if(jr(e,t))return!0;if(typeof e!=`object`||!e||typeof t!=`object`||!t)return!1;var n=Object.keys(e),r=Object.keys(t);if(n.length!==r.length)return!1;for(r=0;r<n.length;r++){var i=n[r];if(!je.call(t,i)||!jr(e[i],t[i]))return!1}return!0}function Nr(e){for(;e&&e.firstChild;)e=e.firstChild;return e}function Pr(e,t){var n=Nr(e);e=0;for(var r;n;){if(n.nodeType===3){if(r=e+n.textContent.length,e<=t&&r>=t)return{node:n,offset:t-e};e=r}a:{for(;n;){if(n.nextSibling){n=n.nextSibling;break a}n=n.parentNode}n=void 0}n=Nr(n)}}function Fr(e,t){return e&&t?e===t?!0:e&&e.nodeType===3?!1:t&&t.nodeType===3?Fr(e,t.parentNode):`contains`in e?e.contains(t):e.compareDocumentPosition?!!(e.compareDocumentPosition(t)&16):!1:!1}function Ir(e){e=e!=null&&e.ownerDocument!=null&&e.ownerDocument.defaultView!=null?e.ownerDocument.defaultView:window;for(var t=qt(e.document);t instanceof e.HTMLIFrameElement;){try{var n=typeof t.contentWindow.location.href==`string`}catch{n=!1}if(n)e=t.contentWindow;else break;t=qt(e.document)}return t}function Lr(e){var t=e&&e.nodeName&&e.nodeName.toLowerCase();return t&&(t===`input`&&(e.type===`text`||e.type===`search`||e.type===`tel`||e.type===`url`||e.type===`password`)||t===`textarea`||e.contentEditable===`true`)}var Rr=yn&&`documentMode`in document&&11>=document.documentMode,zr=null,Br=null,Vr=null,Hr=!1;function Ur(e,t,n){var r=n.window===n?n.document:n.nodeType===9?n:n.ownerDocument;Hr||zr==null||zr!==qt(r)||(r=zr,`selectionStart`in r&&Lr(r)?r={start:r.selectionStart,end:r.selectionEnd}:(r=(r.ownerDocument&&r.ownerDocument.defaultView||window).getSelection(),r={anchorNode:r.anchorNode,anchorOffset:r.anchorOffset,focusNode:r.focusNode,focusOffset:r.focusOffset}),Vr&&Mr(Vr,r)||(Vr=r,r=kd(Br,`onSelect`),0<r.length&&(t=new jn(`onSelect`,`select`,null,t,n),e.push({event:t,listeners:r}),t.target=zr)))}function Wr(e,t){var n={};return n[e.toLowerCase()]=t.toLowerCase(),n[`Webkit`+e]=`webkit`+t,n[`Moz`+e]=`moz`+t,n}var Gr={animationend:Wr(`Animation`,`AnimationEnd`),animationiteration:Wr(`Animation`,`AnimationIteration`),animationstart:Wr(`Animation`,`AnimationStart`),transitionrun:Wr(`Transition`,`TransitionRun`),transitionstart:Wr(`Transition`,`TransitionStart`),transitioncancel:Wr(`Transition`,`TransitionCancel`),transitionend:Wr(`Transition`,`TransitionEnd`)},A={},Kr={};yn&&(Kr=document.createElement(`div`).style,`AnimationEvent`in window||(delete Gr.animationend.animation,delete Gr.animationiteration.animation,delete Gr.animationstart.animation),`TransitionEvent`in window||delete Gr.transitionend.transition);function qr(e){if(A[e])return A[e];if(!Gr[e])return e;var t=Gr[e],n;for(n in t)if(t.hasOwnProperty(n)&&n in Kr)return A[e]=t[n];return e}var Jr=qr(`animationend`),Yr=qr(`animationiteration`),Xr=qr(`animationstart`),Zr=qr(`transitionrun`),Qr=qr(`transitionstart`),$r=qr(`transitioncancel`),ei=qr(`transitionend`),ti=new Map,ni=`abort auxClick beforeToggle cancel canPlay canPlayThrough click close contextMenu copy cut drag dragEnd dragEnter dragExit dragLeave dragOver dragStart drop durationChange emptied encrypted ended error gotPointerCapture input invalid keyDown keyPress keyUp load loadedData loadedMetadata loadStart lostPointerCapture mouseDown mouseMove mouseOut mouseOver mouseUp paste pause play playing pointerCancel pointerDown pointerMove pointerOut pointerOver pointerUp progress rateChange reset resize seeked seeking stalled submit suspend timeUpdate touchCancel touchEnd touchStart volumeChange scroll toggle touchMove waiting wheel`.split(` `);ni.push(`scrollEnd`);function ri(e,t){ti.set(e,t),Nt(t,[e])}var ii=typeof reportError==`function`?reportError:function(e){if(typeof window==`object`&&typeof window.ErrorEvent==`function`){var t=new window.ErrorEvent(`error`,{bubbles:!0,cancelable:!0,message:typeof e==`object`&&e&&typeof e.message==`string`?String(e.message):String(e),error:e});if(!window.dispatchEvent(t))return}else if(typeof process==`object`&&typeof process.emit==`function`){process.emit(`uncaughtException`,e);return}console.error(e)},ai=[],oi=0,si=0;function ci(){for(var e=oi,t=si=oi=0;t<e;){var n=ai[t];ai[t++]=null;var r=ai[t];ai[t++]=null;var i=ai[t];ai[t++]=null;var a=ai[t];if(ai[t++]=null,r!==null&&i!==null){var o=r.pending;o===null?i.next=i:(i.next=o.next,o.next=i),r.pending=i}a!==0&&fi(n,i,a)}}function li(e,t,n,r){ai[oi++]=e,ai[oi++]=t,ai[oi++]=n,ai[oi++]=r,si|=r,e.lanes|=r,e=e.alternate,e!==null&&(e.lanes|=r)}function ui(e,t,n,r){return li(e,t,n,r),pi(e)}function di(e,t){return li(e,null,null,t),pi(e)}function fi(e,t,n){e.lanes|=n;var r=e.alternate;r!==null&&(r.lanes|=n);for(var i=!1,a=e.return;a!==null;)a.childLanes|=n,r=a.alternate,r!==null&&(r.childLanes|=n),a.tag===22&&(e=a.stateNode,e===null||e._visibility&1||(i=!0)),e=a,a=a.return;return e.tag===3?(a=e.stateNode,i&&t!==null&&(i=31-Je(n),e=a.hiddenUpdates,r=e[i],r===null?e[i]=[t]:r.push(t),t.lane=n|536870912),a):null}function pi(e){if(50<mu)throw mu=0,hu=null,Error(s(185));for(var t=e.return;t!==null;)e=t,t=e.return;return e.tag===3?e.stateNode:null}var mi={};function hi(e,t,n,r){this.tag=e,this.key=n,this.sibling=this.child=this.return=this.stateNode=this.type=this.elementType=null,this.index=0,this.refCleanup=this.ref=null,this.pendingProps=t,this.dependencies=this.memoizedState=this.updateQueue=this.memoizedProps=null,this.mode=r,this.subtreeFlags=this.flags=0,this.deletions=null,this.childLanes=this.lanes=0,this.alternate=null}function gi(e,t,n,r){return new hi(e,t,n,r)}function _i(e){return e=e.prototype,!(!e||!e.isReactComponent)}function vi(e,t){var n=e.alternate;return n===null?(n=gi(e.tag,t,e.key,e.mode),n.elementType=e.elementType,n.type=e.type,n.stateNode=e.stateNode,n.alternate=e,e.alternate=n):(n.pendingProps=t,n.type=e.type,n.flags=0,n.subtreeFlags=0,n.deletions=null),n.flags=e.flags&65011712,n.childLanes=e.childLanes,n.lanes=e.lanes,n.child=e.child,n.memoizedProps=e.memoizedProps,n.memoizedState=e.memoizedState,n.updateQueue=e.updateQueue,t=e.dependencies,n.dependencies=t===null?null:{lanes:t.lanes,firstContext:t.firstContext},n.sibling=e.sibling,n.index=e.index,n.ref=e.ref,n.refCleanup=e.refCleanup,n}function yi(e,t){e.flags&=65011714;var n=e.alternate;return n===null?(e.childLanes=0,e.lanes=t,e.child=null,e.subtreeFlags=0,e.memoizedProps=null,e.memoizedState=null,e.updateQueue=null,e.dependencies=null,e.stateNode=null):(e.childLanes=n.childLanes,e.lanes=n.lanes,e.child=n.child,e.subtreeFlags=0,e.deletions=null,e.memoizedProps=n.memoizedProps,e.memoizedState=n.memoizedState,e.updateQueue=n.updateQueue,e.type=n.type,t=n.dependencies,e.dependencies=t===null?null:{lanes:t.lanes,firstContext:t.firstContext}),e}function bi(e,t,n,r,i,a){var o=0;if(r=e,typeof e==`function`)_i(e)&&(o=1);else if(typeof e==`string`)o=Kf(e,n,ge.current)?26:e===`html`||e===`head`||e===`body`?27:5;else a:switch(e){case ie:return e=gi(31,n,t,i),e.elementType=ie,e.lanes=a,e;case y:return xi(n.children,i,a,t);case b:o=8,i|=24;break;case x:return e=gi(12,n,t,i|2),e.elementType=x,e.lanes=a,e;case te:return e=gi(13,n,t,i),e.elementType=te,e.lanes=a,e;case ne:return e=gi(19,n,t,i),e.elementType=ne,e.lanes=a,e;default:if(typeof e==`object`&&e)switch(e.$$typeof){case S:o=10;break a;case ee:o=9;break a;case C:o=11;break a;case w:o=14;break a;case re:o=16,r=null;break a}o=29,n=Error(s(130,e===null?`null`:typeof e,``)),r=null}return t=gi(o,n,t,i),t.elementType=e,t.type=r,t.lanes=a,t}function xi(e,t,n,r){return e=gi(7,e,r,t),e.lanes=n,e}function Si(e,t,n){return e=gi(6,e,null,t),e.lanes=n,e}function Ci(e){var t=gi(18,null,null,0);return t.stateNode=e,t}function wi(e,t,n){return t=gi(4,e.children===null?[]:e.children,e.key,t),t.lanes=n,t.stateNode={containerInfo:e.containerInfo,pendingChildren:null,implementation:e.implementation},t}var Ti=new WeakMap;function Ei(e,t){if(typeof e==`object`&&e){var n=Ti.get(e);return n===void 0?(t={value:e,source:t,stack:Ae(t)},Ti.set(e,t),t):n}return{value:e,source:t,stack:Ae(t)}}var Di=[],Oi=0,ki=null,Ai=0,ji=[],Mi=0,Ni=null,Pi=1,Fi=``;function Ii(e,t){Di[Oi++]=Ai,Di[Oi++]=ki,ki=e,Ai=t}function Li(e,t,n){ji[Mi++]=Pi,ji[Mi++]=Fi,ji[Mi++]=Ni,Ni=e;var r=Pi;e=Fi;var i=32-Je(r)-1;r&=~(1<<i),n+=1;var a=32-Je(t)+i;if(30<a){var o=i-i%5;a=(r&(1<<o)-1).toString(32),r>>=o,i-=o,Pi=1<<32-Je(t)+i|n<<i|r,Fi=a+e}else Pi=1<<a|n<<i|r,Fi=e}function Ri(e){e.return!==null&&(Ii(e,1),Li(e,1,0))}function zi(e){for(;e===ki;)ki=Di[--Oi],Di[Oi]=null,Ai=Di[--Oi],Di[Oi]=null;for(;e===Ni;)Ni=ji[--Mi],ji[Mi]=null,Fi=ji[--Mi],ji[Mi]=null,Pi=ji[--Mi],ji[Mi]=null}function Bi(e,t){ji[Mi++]=Pi,ji[Mi++]=Fi,ji[Mi++]=Ni,Pi=t.id,Fi=t.overflow,Ni=e}var Vi=null,Hi=null,j=!1,Ui=null,Wi=!1,Gi=Error(s(519));function Ki(e){throw Qi(Ei(Error(s(418,1<arguments.length&&arguments[1]!==void 0&&arguments[1]?`text`:`HTML`,``)),e)),Gi}function qi(e){var t=e.stateNode,n=e.type,r=e.memoizedProps;switch(t[_t]=e,t[vt]=r,n){case`dialog`:Y(`cancel`,t),Y(`close`,t);break;case`iframe`:case`object`:case`embed`:Y(`load`,t);break;case`video`:case`audio`:for(n=0;n<bd.length;n++)Y(bd[n],t);break;case`source`:Y(`error`,t);break;case`img`:case`image`:case`link`:Y(`error`,t),Y(`load`,t);break;case`details`:Y(`toggle`,t);break;case`input`:Y(`invalid`,t),Zt(t,r.value,r.defaultValue,r.checked,r.defaultChecked,r.type,r.name,!0);break;case`select`:Y(`invalid`,t);break;case`textarea`:Y(`invalid`,t),tn(t,r.value,r.defaultValue,r.children)}n=r.children,typeof n!=`string`&&typeof n!=`number`&&typeof n!=`bigint`||t.textContent===``+n||!0===r.suppressHydrationWarning||Fd(t.textContent,n)?(r.popover!=null&&(Y(`beforetoggle`,t),Y(`toggle`,t)),r.onScroll!=null&&Y(`scroll`,t),r.onScrollEnd!=null&&Y(`scrollend`,t),r.onClick!=null&&(t.onclick=un),t=!0):t=!1,t||Ki(e,!0)}function Ji(e){for(Vi=e.return;Vi;)switch(Vi.tag){case 5:case 31:case 13:Wi=!1;return;case 27:case 3:Wi=!0;return;default:Vi=Vi.return}}function Yi(e){if(e!==Vi)return!1;if(!j)return Ji(e),j=!0,!1;var t=e.tag,n;if((n=t!==3&&t!==27)&&((n=t===5)&&(n=e.type,n=!(n!==`form`&&n!==`button`)||qd(e.type,e.memoizedProps)),n=!n),n&&Hi&&Ki(e),Ji(e),t===13){if(e=e.memoizedState,e=e===null?null:e.dehydrated,!e)throw Error(s(317));Hi=pf(e)}else if(t===31){if(e=e.memoizedState,e=e===null?null:e.dehydrated,!e)throw Error(s(317));Hi=pf(e)}else t===27?(t=Hi,tf(e.type)?(e=ff,ff=null,Hi=e):Hi=t):Hi=Vi?df(e.stateNode.nextSibling):null;return!0}function Xi(){Hi=Vi=null,j=!1}function Zi(){var e=Ui;return e!==null&&(tu===null?tu=e:tu.push.apply(tu,e),Ui=null),e}function Qi(e){Ui===null?Ui=[e]:Ui.push(e)}var $i=pe(null),ea=null,ta=null;function na(e,t,n){he($i,t._currentValue),t._currentValue=n}function ra(e){e._currentValue=$i.current,me($i)}function ia(e,t,n){for(;e!==null;){var r=e.alternate;if((e.childLanes&t)===t?r!==null&&(r.childLanes&t)!==t&&(r.childLanes|=t):(e.childLanes|=t,r!==null&&(r.childLanes|=t)),e===n)break;e=e.return}}function M(e,t,n,r){var i=e.child;for(i!==null&&(i.return=e);i!==null;){var a=i.dependencies;if(a!==null){var o=i.child;a=a.firstContext;a:for(;a!==null;){var c=a;a=i;for(var l=0;l<t.length;l++)if(c.context===t[l]){a.lanes|=n,c=a.alternate,c!==null&&(c.lanes|=n),ia(a.return,n,e),r||(o=null);break a}a=c.next}}else if(i.tag===18){if(o=i.return,o===null)throw Error(s(341));o.lanes|=n,a=o.alternate,a!==null&&(a.lanes|=n),ia(o,n,e),o=null}else o=i.child;if(o!==null)o.return=i;else for(o=i;o!==null;){if(o===e){o=null;break}if(i=o.sibling,i!==null){i.return=o.return,o=i;break}o=o.return}i=o}}function aa(e,t,n,r){e=null;for(var i=t,a=!1;i!==null;){if(!a){if(i.flags&524288)a=!0;else if(i.flags&262144)break}if(i.tag===10){var o=i.alternate;if(o===null)throw Error(s(387));if(o=o.memoizedProps,o!==null){var c=i.type;jr(i.pendingProps.value,o.value)||(e===null?e=[c]:e.push(c))}}else if(i===ye.current){if(o=i.alternate,o===null)throw Error(s(387));o.memoizedState.memoizedState!==i.memoizedState.memoizedState&&(e===null?e=[tp]:e.push(tp))}i=i.return}e!==null&&M(t,e,n,r),t.flags|=262144}function oa(e){for(e=e.firstContext;e!==null;){if(!jr(e.context._currentValue,e.memoizedValue))return!0;e=e.next}return!1}function sa(e){ea=e,ta=null,e=e.dependencies,e!==null&&(e.firstContext=null)}function ca(e){return ua(ea,e)}function la(e,t){return ea===null&&sa(e),ua(e,t)}function ua(e,t){var n=t._currentValue;if(t={context:t,memoizedValue:n,next:null},ta===null){if(e===null)throw Error(s(308));ta=t,e.dependencies={lanes:0,firstContext:t},e.flags|=524288}else ta=ta.next=t;return n}var da=typeof AbortController<`u`?AbortController:function(){var e=[],t=this.signal={aborted:!1,addEventListener:function(t,n){e.push(n)}};this.abort=function(){t.aborted=!0,e.forEach(function(e){return e()})}},fa=t.unstable_scheduleCallback,pa=t.unstable_NormalPriority,ma={$$typeof:S,Consumer:null,Provider:null,_currentValue:null,_currentValue2:null,_threadCount:0};function ha(){return{controller:new da,data:new Map,refCount:0}}function ga(e){e.refCount--,e.refCount===0&&fa(pa,function(){e.controller.abort()})}var _a=null,va=0,ya=0,ba=null;function xa(e,t){if(_a===null){var n=_a=[];va=0,ya=hd(),ba={status:`pending`,value:void 0,then:function(e){n.push(e)}}}return va++,t.then(Sa,Sa),t}function Sa(){if(--va===0&&_a!==null){ba!==null&&(ba.status=`fulfilled`);var e=_a;_a=null,ya=0,ba=null;for(var t=0;t<e.length;t++)(0,e[t])()}}function Ca(e,t){var n=[],r={status:`pending`,value:null,reason:null,then:function(e){n.push(e)}};return e.then(function(){r.status=`fulfilled`,r.value=t;for(var e=0;e<n.length;e++)(0,n[e])(t)},function(e){for(r.status=`rejected`,r.reason=e,e=0;e<n.length;e++)(0,n[e])(void 0)}),r}var wa=E.S;E.S=function(e,t){iu=Ie(),typeof t==`object`&&t&&typeof t.then==`function`&&xa(e,t),wa!==null&&wa(e,t)};var Ta=pe(null);function Ea(){var e=Ta.current;return e===null?U.pooledCache:e}function Da(e,t){t===null?he(Ta,Ta.current):he(Ta,t.pool)}function Oa(){var e=Ea();return e===null?null:{parent:ma._currentValue,pool:e}}var ka=Error(s(460)),Aa=Error(s(474)),ja=Error(s(542)),Ma={then:function(){}};function Na(e){return e=e.status,e===`fulfilled`||e===`rejected`}function Pa(e,t,n){switch(n=e[n],n===void 0?e.push(t):n!==t&&(t.then(un,un),t=n),t.status){case`fulfilled`:return t.value;case`rejected`:throw e=t.reason,Ra(e),e;default:if(typeof t.status==`string`)t.then(un,un);else{if(e=U,e!==null&&100<e.shellSuspendCounter)throw Error(s(482));e=t,e.status=`pending`,e.then(function(e){if(t.status===`pending`){var n=t;n.status=`fulfilled`,n.value=e}},function(e){if(t.status===`pending`){var n=t;n.status=`rejected`,n.reason=e}})}switch(t.status){case`fulfilled`:return t.value;case`rejected`:throw e=t.reason,Ra(e),e}throw Ia=t,ka}}function Fa(e){try{var t=e._init;return t(e._payload)}catch(e){throw typeof e==`object`&&e&&typeof e.then==`function`?(Ia=e,ka):e}}var Ia=null;function La(){if(Ia===null)throw Error(s(459));var e=Ia;return Ia=null,e}function Ra(e){if(e===ka||e===ja)throw Error(s(483))}var za=null,Ba=0;function Va(e){var t=Ba;return Ba+=1,za===null&&(za=[]),Pa(za,e,t)}function Ha(e,t){t=t.props.ref,e.ref=t===void 0?null:t}function Ua(e,t){throw t.$$typeof===g?Error(s(525)):(e=Object.prototype.toString.call(t),Error(s(31,e===`[object Object]`?`object with keys {`+Object.keys(t).join(`, `)+`}`:e)))}function Wa(e){function t(t,n){if(e){var r=t.deletions;r===null?(t.deletions=[n],t.flags|=16):r.push(n)}}function n(n,r){if(!e)return null;for(;r!==null;)t(n,r),r=r.sibling;return null}function r(e){for(var t=new Map;e!==null;)e.key===null?t.set(e.index,e):t.set(e.key,e),e=e.sibling;return t}function i(e,t){return e=vi(e,t),e.index=0,e.sibling=null,e}function a(t,n,r){return t.index=r,e?(r=t.alternate,r===null?(t.flags|=67108866,n):(r=r.index,r<n?(t.flags|=67108866,n):r)):(t.flags|=1048576,n)}function o(t){return e&&t.alternate===null&&(t.flags|=67108866),t}function c(e,t,n,r){return t===null||t.tag!==6?(t=Si(n,e.mode,r),t.return=e,t):(t=i(t,n),t.return=e,t)}function l(e,t,n,r){var a=n.type;return a===y?d(e,t,n.props.children,r,n.key):t!==null&&(t.elementType===a||typeof a==`object`&&a&&a.$$typeof===re&&Fa(a)===t.type)?(t=i(t,n.props),Ha(t,n),t.return=e,t):(t=bi(n.type,n.key,n.props,null,e.mode,r),Ha(t,n),t.return=e,t)}function u(e,t,n,r){return t===null||t.tag!==4||t.stateNode.containerInfo!==n.containerInfo||t.stateNode.implementation!==n.implementation?(t=wi(n,e.mode,r),t.return=e,t):(t=i(t,n.children||[]),t.return=e,t)}function d(e,t,n,r,a){return t===null||t.tag!==7?(t=xi(n,e.mode,r,a),t.return=e,t):(t=i(t,n),t.return=e,t)}function f(e,t,n){if(typeof t==`string`&&t!==``||typeof t==`number`||typeof t==`bigint`)return t=Si(``+t,e.mode,n),t.return=e,t;if(typeof t==`object`&&t){switch(t.$$typeof){case _:return n=bi(t.type,t.key,t.props,null,e.mode,n),Ha(n,t),n.return=e,n;case v:return t=wi(t,e.mode,n),t.return=e,t;case re:return t=Fa(t),f(e,t,n)}if(le(t)||oe(t))return t=xi(t,e.mode,n,null),t.return=e,t;if(typeof t.then==`function`)return f(e,Va(t),n);if(t.$$typeof===S)return f(e,la(e,t),n);Ua(e,t)}return null}function p(e,t,n,r){var i=t===null?null:t.key;if(typeof n==`string`&&n!==``||typeof n==`number`||typeof n==`bigint`)return i===null?c(e,t,``+n,r):null;if(typeof n==`object`&&n){switch(n.$$typeof){case _:return n.key===i?l(e,t,n,r):null;case v:return n.key===i?u(e,t,n,r):null;case re:return n=Fa(n),p(e,t,n,r)}if(le(n)||oe(n))return i===null?d(e,t,n,r,null):null;if(typeof n.then==`function`)return p(e,t,Va(n),r);if(n.$$typeof===S)return p(e,t,la(e,n),r);Ua(e,n)}return null}function m(e,t,n,r,i){if(typeof r==`string`&&r!==``||typeof r==`number`||typeof r==`bigint`)return e=e.get(n)||null,c(t,e,``+r,i);if(typeof r==`object`&&r){switch(r.$$typeof){case _:return e=e.get(r.key===null?n:r.key)||null,l(t,e,r,i);case v:return e=e.get(r.key===null?n:r.key)||null,u(t,e,r,i);case re:return r=Fa(r),m(e,t,n,r,i)}if(le(r)||oe(r))return e=e.get(n)||null,d(t,e,r,i,null);if(typeof r.then==`function`)return m(e,t,n,Va(r),i);if(r.$$typeof===S)return m(e,t,n,la(t,r),i);Ua(t,r)}return null}function h(i,o,s,c){for(var l=null,u=null,d=o,h=o=0,g=null;d!==null&&h<s.length;h++){d.index>h?(g=d,d=null):g=d.sibling;var _=p(i,d,s[h],c);if(_===null){d===null&&(d=g);break}e&&d&&_.alternate===null&&t(i,d),o=a(_,o,h),u===null?l=_:u.sibling=_,u=_,d=g}if(h===s.length)return n(i,d),j&&Ii(i,h),l;if(d===null){for(;h<s.length;h++)d=f(i,s[h],c),d!==null&&(o=a(d,o,h),u===null?l=d:u.sibling=d,u=d);return j&&Ii(i,h),l}for(d=r(d);h<s.length;h++)g=m(d,i,h,s[h],c),g!==null&&(e&&g.alternate!==null&&d.delete(g.key===null?h:g.key),o=a(g,o,h),u===null?l=g:u.sibling=g,u=g);return e&&d.forEach(function(e){return t(i,e)}),j&&Ii(i,h),l}function g(i,o,c,l){if(c==null)throw Error(s(151));for(var u=null,d=null,h=o,g=o=0,_=null,v=c.next();h!==null&&!v.done;g++,v=c.next()){h.index>g?(_=h,h=null):_=h.sibling;var y=p(i,h,v.value,l);if(y===null){h===null&&(h=_);break}e&&h&&y.alternate===null&&t(i,h),o=a(y,o,g),d===null?u=y:d.sibling=y,d=y,h=_}if(v.done)return n(i,h),j&&Ii(i,g),u;if(h===null){for(;!v.done;g++,v=c.next())v=f(i,v.value,l),v!==null&&(o=a(v,o,g),d===null?u=v:d.sibling=v,d=v);return j&&Ii(i,g),u}for(h=r(h);!v.done;g++,v=c.next())v=m(h,i,g,v.value,l),v!==null&&(e&&v.alternate!==null&&h.delete(v.key===null?g:v.key),o=a(v,o,g),d===null?u=v:d.sibling=v,d=v);return e&&h.forEach(function(e){return t(i,e)}),j&&Ii(i,g),u}function b(e,r,a,c){if(typeof a==`object`&&a&&a.type===y&&a.key===null&&(a=a.props.children),typeof a==`object`&&a){switch(a.$$typeof){case _:a:{for(var l=a.key;r!==null;){if(r.key===l){if(l=a.type,l===y){if(r.tag===7){n(e,r.sibling),c=i(r,a.props.children),c.return=e,e=c;break a}}else if(r.elementType===l||typeof l==`object`&&l&&l.$$typeof===re&&Fa(l)===r.type){n(e,r.sibling),c=i(r,a.props),Ha(c,a),c.return=e,e=c;break a}n(e,r);break}else t(e,r);r=r.sibling}a.type===y?(c=xi(a.props.children,e.mode,c,a.key),c.return=e,e=c):(c=bi(a.type,a.key,a.props,null,e.mode,c),Ha(c,a),c.return=e,e=c)}return o(e);case v:a:{for(l=a.key;r!==null;){if(r.key===l)if(r.tag===4&&r.stateNode.containerInfo===a.containerInfo&&r.stateNode.implementation===a.implementation){n(e,r.sibling),c=i(r,a.children||[]),c.return=e,e=c;break a}else{n(e,r);break}else t(e,r);r=r.sibling}c=wi(a,e.mode,c),c.return=e,e=c}return o(e);case re:return a=Fa(a),b(e,r,a,c)}if(le(a))return h(e,r,a,c);if(oe(a)){if(l=oe(a),typeof l!=`function`)throw Error(s(150));return a=l.call(a),g(e,r,a,c)}if(typeof a.then==`function`)return b(e,r,Va(a),c);if(a.$$typeof===S)return b(e,r,la(e,a),c);Ua(e,a)}return typeof a==`string`&&a!==``||typeof a==`number`||typeof a==`bigint`?(a=``+a,r!==null&&r.tag===6?(n(e,r.sibling),c=i(r,a),c.return=e,e=c):(n(e,r),c=Si(a,e.mode,c),c.return=e,e=c),o(e)):n(e,r)}return function(e,t,n,r){try{Ba=0;var i=b(e,t,n,r);return za=null,i}catch(t){if(t===ka||t===ja)throw t;var a=gi(29,t,null,e.mode);return a.lanes=r,a.return=e,a}}}var Ga=Wa(!0),Ka=Wa(!1),qa=!1;function Ja(e){e.updateQueue={baseState:e.memoizedState,firstBaseUpdate:null,lastBaseUpdate:null,shared:{pending:null,lanes:0,hiddenCallbacks:null},callbacks:null}}function Ya(e,t){e=e.updateQueue,t.updateQueue===e&&(t.updateQueue={baseState:e.baseState,firstBaseUpdate:e.firstBaseUpdate,lastBaseUpdate:e.lastBaseUpdate,shared:e.shared,callbacks:null})}function Xa(e){return{lane:e,tag:0,payload:null,callback:null,next:null}}function Za(e,t,n){var r=e.updateQueue;if(r===null)return null;if(r=r.shared,H&2){var i=r.pending;return i===null?t.next=t:(t.next=i.next,i.next=t),r.pending=t,t=pi(e),fi(e,null,n),t}return li(e,r,t,n),pi(e)}function Qa(e,t,n){if(t=t.updateQueue,t!==null&&(t=t.shared,n&4194048)){var r=t.lanes;r&=e.pendingLanes,n|=r,t.lanes=n,ut(e,n)}}function $a(e,t){var n=e.updateQueue,r=e.alternate;if(r!==null&&(r=r.updateQueue,n===r)){var i=null,a=null;if(n=n.firstBaseUpdate,n!==null){do{var o={lane:n.lane,tag:n.tag,payload:n.payload,callback:null,next:null};a===null?i=a=o:a=a.next=o,n=n.next}while(n!==null);a===null?i=a=t:a=a.next=t}else i=a=t;n={baseState:r.baseState,firstBaseUpdate:i,lastBaseUpdate:a,shared:r.shared,callbacks:r.callbacks},e.updateQueue=n;return}e=n.lastBaseUpdate,e===null?n.firstBaseUpdate=t:e.next=t,n.lastBaseUpdate=t}var eo=!1;function to(){if(eo){var e=ba;if(e!==null)throw e}}function no(e,t,n,r){eo=!1;var i=e.updateQueue;qa=!1;var a=i.firstBaseUpdate,o=i.lastBaseUpdate,s=i.shared.pending;if(s!==null){i.shared.pending=null;var c=s,l=c.next;c.next=null,o===null?a=l:o.next=l,o=c;var u=e.alternate;u!==null&&(u=u.updateQueue,s=u.lastBaseUpdate,s!==o&&(s===null?u.firstBaseUpdate=l:s.next=l,u.lastBaseUpdate=c))}if(a!==null){var d=i.baseState;o=0,u=l=c=null,s=a;do{var f=s.lane&-536870913,p=f!==s.lane;if(p?(G&f)===f:(r&f)===f){f!==0&&f===ya&&(eo=!0),u!==null&&(u=u.next={lane:0,tag:s.tag,payload:s.payload,callback:null,next:null});a:{var m=e,g=s;f=t;var _=n;switch(g.tag){case 1:if(m=g.payload,typeof m==`function`){d=m.call(_,d,f);break a}d=m;break a;case 3:m.flags=m.flags&-65537|128;case 0:if(m=g.payload,f=typeof m==`function`?m.call(_,d,f):m,f==null)break a;d=h({},d,f);break a;case 2:qa=!0}}f=s.callback,f!==null&&(e.flags|=64,p&&(e.flags|=8192),p=i.callbacks,p===null?i.callbacks=[f]:p.push(f))}else p={lane:f,tag:s.tag,payload:s.payload,callback:s.callback,next:null},u===null?(l=u=p,c=d):u=u.next=p,o|=f;if(s=s.next,s===null){if(s=i.shared.pending,s===null)break;p=s,s=p.next,p.next=null,i.lastBaseUpdate=p,i.shared.pending=null}}while(1);u===null&&(c=d),i.baseState=c,i.firstBaseUpdate=l,i.lastBaseUpdate=u,a===null&&(i.shared.lanes=0),Yl|=o,e.lanes=o,e.memoizedState=d}}function ro(e,t){if(typeof e!=`function`)throw Error(s(191,e));e.call(t)}function io(e,t){var n=e.callbacks;if(n!==null)for(e.callbacks=null,e=0;e<n.length;e++)ro(n[e],t)}var ao=pe(null),oo=pe(0);function so(e,t){e=ql,he(oo,e),he(ao,t),ql=e|t.baseLanes}function co(){he(oo,ql),he(ao,ao.current)}function lo(){ql=oo.current,me(ao),me(oo)}var uo=pe(null),fo=null;function po(e){var t=e.alternate;he(_o,_o.current&1),he(uo,e),fo===null&&(t===null||ao.current!==null||t.memoizedState!==null)&&(fo=e)}function N(e){he(_o,_o.current),he(uo,e),fo===null&&(fo=e)}function mo(e){e.tag===22?(he(_o,_o.current),he(uo,e),fo===null&&(fo=e)):ho(e)}function ho(){he(_o,_o.current),he(uo,uo.current)}function go(e){me(uo),fo===e&&(fo=null),me(_o)}var _o=pe(0);function vo(e){for(var t=e;t!==null;){if(t.tag===13){var n=t.memoizedState;if(n!==null&&(n=n.dehydrated,n===null||cf(n)||lf(n)))return t}else if(t.tag===19&&(t.memoizedProps.revealOrder===`forwards`||t.memoizedProps.revealOrder===`backwards`||t.memoizedProps.revealOrder===`unstable_legacy-backwards`||t.memoizedProps.revealOrder===`together`)){if(t.flags&128)return t}else if(t.child!==null){t.child.return=t,t=t.child;continue}if(t===e)break;for(;t.sibling===null;){if(t.return===null||t.return===e)return null;t=t.return}t.sibling.return=t.return,t=t.sibling}return null}var yo=0,P=null,bo=null,xo=null,So=!1,Co=!1,wo=!1,To=0,Eo=0,Do=null,Oo=0;function ko(){throw Error(s(321))}function Ao(e,t){if(t===null)return!1;for(var n=0;n<t.length&&n<e.length;n++)if(!jr(e[n],t[n]))return!1;return!0}function jo(e,t,n,r,i,a){return yo=a,P=t,t.memoizedState=null,t.updateQueue=null,t.lanes=0,E.H=e===null||e.memoizedState===null?Ks:qs,wo=!1,a=n(r,i),wo=!1,Co&&(a=No(t,n,r,i)),Mo(e),a}function Mo(e){E.H=Gs;var t=bo!==null&&bo.next!==null;if(yo=0,xo=bo=P=null,So=!1,Eo=0,Do=null,t)throw Error(s(300));e===null||uc||(e=e.dependencies,e!==null&&oa(e)&&(uc=!0))}function No(e,t,n,r){P=e;var i=0;do{if(Co&&(Do=null),Eo=0,Co=!1,25<=i)throw Error(s(301));if(i+=1,xo=bo=null,e.updateQueue!=null){var a=e.updateQueue;a.lastEffect=null,a.events=null,a.stores=null,a.memoCache!=null&&(a.memoCache.index=0)}E.H=Js,a=t(n,r)}while(Co);return a}function Po(){var e=E.H,t=e.useState()[0];return t=typeof t.then==`function`?Vo(t):t,e=e.useState()[0],(bo===null?null:bo.memoizedState)!==e&&(P.flags|=1024),t}function Fo(){var e=To!==0;return To=0,e}function Io(e,t,n){t.updateQueue=e.updateQueue,t.flags&=-2053,e.lanes&=~n}function Lo(e){if(So){for(e=e.memoizedState;e!==null;){var t=e.queue;t!==null&&(t.pending=null),e=e.next}So=!1}yo=0,xo=bo=P=null,Co=!1,Eo=To=0,Do=null}function Ro(){var e={memoizedState:null,baseState:null,baseQueue:null,queue:null,next:null};return xo===null?P.memoizedState=xo=e:xo=xo.next=e,xo}function zo(){if(bo===null){var e=P.alternate;e=e===null?null:e.memoizedState}else e=bo.next;var t=xo===null?P.memoizedState:xo.next;if(t!==null)xo=t,bo=e;else{if(e===null)throw P.alternate===null?Error(s(467)):Error(s(310));bo=e,e={memoizedState:bo.memoizedState,baseState:bo.baseState,baseQueue:bo.baseQueue,queue:bo.queue,next:null},xo===null?P.memoizedState=xo=e:xo=xo.next=e}return xo}function Bo(){return{lastEffect:null,events:null,stores:null,memoCache:null}}function Vo(e){var t=Eo;return Eo+=1,Do===null&&(Do=[]),e=Pa(Do,e,t),t=P,(xo===null?t.memoizedState:xo.next)===null&&(t=t.alternate,E.H=t===null||t.memoizedState===null?Ks:qs),e}function Ho(e){if(typeof e==`object`&&e){if(typeof e.then==`function`)return Vo(e);if(e.$$typeof===S)return ca(e)}throw Error(s(438,String(e)))}function Uo(e){var t=null,n=P.updateQueue;if(n!==null&&(t=n.memoCache),t==null){var r=P.alternate;r!==null&&(r=r.updateQueue,r!==null&&(r=r.memoCache,r!=null&&(t={data:r.data.map(function(e){return e.slice()}),index:0})))}if(t??={data:[],index:0},n===null&&(n=Bo(),P.updateQueue=n),n.memoCache=t,n=t.data[t.index],n===void 0)for(n=t.data[t.index]=Array(e),r=0;r<e;r++)n[r]=ae;return t.index++,n}function Wo(e,t){return typeof t==`function`?t(e):t}function Go(e){return Ko(zo(),bo,e)}function Ko(e,t,n){var r=e.queue;if(r===null)throw Error(s(311));r.lastRenderedReducer=n;var i=e.baseQueue,a=r.pending;if(a!==null){if(i!==null){var o=i.next;i.next=a.next,a.next=o}t.baseQueue=i=a,r.pending=null}if(a=e.baseState,i===null)e.memoizedState=a;else{t=i.next;var c=o=null,l=null,u=t,d=!1;do{var f=u.lane&-536870913;if(f===u.lane?(yo&f)===f:(G&f)===f){var p=u.revertLane;if(p===0)l!==null&&(l=l.next={lane:0,revertLane:0,gesture:null,action:u.action,hasEagerState:u.hasEagerState,eagerState:u.eagerState,next:null}),f===ya&&(d=!0);else if((yo&p)===p){u=u.next,p===ya&&(d=!0);continue}else f={lane:0,revertLane:u.revertLane,gesture:null,action:u.action,hasEagerState:u.hasEagerState,eagerState:u.eagerState,next:null},l===null?(c=l=f,o=a):l=l.next=f,P.lanes|=p,Yl|=p;f=u.action,wo&&n(a,f),a=u.hasEagerState?u.eagerState:n(a,f)}else p={lane:f,revertLane:u.revertLane,gesture:u.gesture,action:u.action,hasEagerState:u.hasEagerState,eagerState:u.eagerState,next:null},l===null?(c=l=p,o=a):l=l.next=p,P.lanes|=f,Yl|=f;u=u.next}while(u!==null&&u!==t);if(l===null?o=a:l.next=c,!jr(a,e.memoizedState)&&(uc=!0,d&&(n=ba,n!==null)))throw n;e.memoizedState=a,e.baseState=o,e.baseQueue=l,r.lastRenderedState=a}return i===null&&(r.lanes=0),[e.memoizedState,r.dispatch]}function qo(e){var t=zo(),n=t.queue;if(n===null)throw Error(s(311));n.lastRenderedReducer=e;var r=n.dispatch,i=n.pending,a=t.memoizedState;if(i!==null){n.pending=null;var o=i=i.next;do a=e(a,o.action),o=o.next;while(o!==i);jr(a,t.memoizedState)||(uc=!0),t.memoizedState=a,t.baseQueue===null&&(t.baseState=a),n.lastRenderedState=a}return[a,r]}function Jo(e,t,n){var r=P,i=zo(),a=j;if(a){if(n===void 0)throw Error(s(407));n=n()}else n=t();var o=!jr((bo||i).memoizedState,n);if(o&&(i.memoizedState=n,uc=!0),i=i.queue,vs(Zo.bind(null,r,i,e),[e]),i.getSnapshot!==t||o||xo!==null&&xo.memoizedState.tag&1){if(r.flags|=2048,ms(9,{destroy:void 0},Xo.bind(null,r,i,n,t),null),U===null)throw Error(s(349));a||yo&127||Yo(r,t,n)}return n}function Yo(e,t,n){e.flags|=16384,e={getSnapshot:t,value:n},t=P.updateQueue,t===null?(t=Bo(),P.updateQueue=t,t.stores=[e]):(n=t.stores,n===null?t.stores=[e]:n.push(e))}function Xo(e,t,n,r){t.value=n,t.getSnapshot=r,Qo(t)&&$o(e)}function Zo(e,t,n){return n(function(){Qo(t)&&$o(e)})}function Qo(e){var t=e.getSnapshot;e=e.value;try{var n=t();return!jr(e,n)}catch{return!0}}function $o(e){var t=di(e,2);t!==null&&vu(t,e,2)}function es(e){var t=Ro();if(typeof e==`function`){var n=e;if(e=n(),wo){qe(!0);try{n()}finally{qe(!1)}}}return t.memoizedState=t.baseState=e,t.queue={pending:null,lanes:0,dispatch:null,lastRenderedReducer:Wo,lastRenderedState:e},t}function ts(e,t,n,r){return e.baseState=n,Ko(e,bo,typeof r==`function`?r:Wo)}function ns(e,t,n,r,i){if(Hs(e))throw Error(s(485));if(e=t.action,e!==null){var a={payload:i,action:e,next:null,isTransition:!0,status:`pending`,value:null,reason:null,listeners:[],then:function(e){a.listeners.push(e)}};E.T===null?a.isTransition=!1:n(!0),r(a),n=t.pending,n===null?(a.next=t.pending=a,rs(t,a)):(a.next=n.next,t.pending=n.next=a)}}function rs(e,t){var n=t.action,r=t.payload,i=e.state;if(t.isTransition){var a=E.T,o={};E.T=o;try{var s=n(i,r),c=E.S;c!==null&&c(o,s),is(e,t,s)}catch(n){os(e,t,n)}finally{a!==null&&o.types!==null&&(a.types=o.types),E.T=a}}else try{a=n(i,r),is(e,t,a)}catch(n){os(e,t,n)}}function is(e,t,n){typeof n==`object`&&n&&typeof n.then==`function`?n.then(function(n){as(e,t,n)},function(n){return os(e,t,n)}):as(e,t,n)}function as(e,t,n){t.status=`fulfilled`,t.value=n,ss(t),e.state=n,t=e.pending,t!==null&&(n=t.next,n===t?e.pending=null:(n=n.next,t.next=n,rs(e,n)))}function os(e,t,n){var r=e.pending;if(e.pending=null,r!==null){r=r.next;do t.status=`rejected`,t.reason=n,ss(t),t=t.next;while(t!==r)}e.action=null}function ss(e){e=e.listeners;for(var t=0;t<e.length;t++)(0,e[t])()}function cs(e,t){return t}function ls(e,t){if(j){var n=U.formState;if(n!==null){a:{var r=P;if(j){if(Hi){b:{for(var i=Hi,a=Wi;i.nodeType!==8;){if(!a){i=null;break b}if(i=df(i.nextSibling),i===null){i=null;break b}}a=i.data,i=a===`F!`||a===`F`?i:null}if(i){Hi=df(i.nextSibling),r=i.data===`F!`;break a}}Ki(r)}r=!1}r&&(t=n[0])}}return n=Ro(),n.memoizedState=n.baseState=t,r={pending:null,lanes:0,dispatch:null,lastRenderedReducer:cs,lastRenderedState:t},n.queue=r,n=zs.bind(null,P,r),r.dispatch=n,r=es(!1),a=Vs.bind(null,P,!1,r.queue),r=Ro(),i={state:t,dispatch:null,action:e,pending:null},r.queue=i,n=ns.bind(null,P,i,a,n),i.dispatch=n,r.memoizedState=e,[t,n,!1]}function us(e){return ds(zo(),bo,e)}function ds(e,t,n){if(t=Ko(e,t,cs)[0],e=Go(Wo)[0],typeof t==`object`&&t&&typeof t.then==`function`)try{var r=Vo(t)}catch(e){throw e===ka?ja:e}else r=t;t=zo();var i=t.queue,a=i.dispatch;return n!==t.memoizedState&&(P.flags|=2048,ms(9,{destroy:void 0},fs.bind(null,i,n),null)),[r,a,e]}function fs(e,t){e.action=t}function ps(e){var t=zo(),n=bo;if(n!==null)return ds(t,n,e);zo(),t=t.memoizedState,n=zo();var r=n.queue.dispatch;return n.memoizedState=e,[t,r,!1]}function ms(e,t,n,r){return e={tag:e,create:n,deps:r,inst:t,next:null},t=P.updateQueue,t===null&&(t=Bo(),P.updateQueue=t),n=t.lastEffect,n===null?t.lastEffect=e.next=e:(r=n.next,n.next=e,e.next=r,t.lastEffect=e),e}function hs(){return zo().memoizedState}function gs(e,t,n,r){var i=Ro();P.flags|=e,i.memoizedState=ms(1|t,{destroy:void 0},n,r===void 0?null:r)}function F(e,t,n,r){var i=zo();r=r===void 0?null:r;var a=i.memoizedState.inst;bo!==null&&r!==null&&Ao(r,bo.memoizedState.deps)?i.memoizedState=ms(t,a,n,r):(P.flags|=e,i.memoizedState=ms(1|t,a,n,r))}function _s(e,t){gs(8390656,8,e,t)}function vs(e,t){F(2048,8,e,t)}function I(e){P.flags|=4;var t=P.updateQueue;if(t===null)t=Bo(),P.updateQueue=t,t.events=[e];else{var n=t.events;n===null?t.events=[e]:n.push(e)}}function ys(e){var t=zo().memoizedState;return I({ref:t,nextImpl:e}),function(){if(H&2)throw Error(s(440));return t.impl.apply(void 0,arguments)}}function bs(e,t){return F(4,2,e,t)}function xs(e,t){return F(4,4,e,t)}function Ss(e,t){if(typeof t==`function`){e=e();var n=t(e);return function(){typeof n==`function`?n():t(null)}}if(t!=null)return e=e(),t.current=e,function(){t.current=null}}function Cs(e,t,n){n=n==null?null:n.concat([e]),F(4,4,Ss.bind(null,t,e),n)}function ws(){}function Ts(e,t){var n=zo();t=t===void 0?null:t;var r=n.memoizedState;return t!==null&&Ao(t,r[1])?r[0]:(n.memoizedState=[e,t],e)}function Es(e,t){var n=zo();t=t===void 0?null:t;var r=n.memoizedState;if(t!==null&&Ao(t,r[1]))return r[0];if(r=e(),wo){qe(!0);try{e()}finally{qe(!1)}}return n.memoizedState=[r,t],r}function Ds(e,t,n){return n===void 0||yo&1073741824&&!(G&261930)?e.memoizedState=t:(e.memoizedState=n,e=_u(),P.lanes|=e,Yl|=e,n)}function Os(e,t,n,r){return jr(n,t)?n:ao.current===null?!(yo&42)||yo&1073741824&&!(G&261930)?(uc=!0,e.memoizedState=n):(e=_u(),P.lanes|=e,Yl|=e,t):(e=Ds(e,n,r),jr(e,t)||(uc=!0),e)}function ks(e,t,n,r,i){var a=D.p;D.p=a!==0&&8>a?a:8;var o=E.T,s={};E.T=s,Vs(e,!1,t,n);try{var c=i(),l=E.S;l!==null&&l(s,c),typeof c==`object`&&c&&typeof c.then==`function`?Bs(e,t,Ca(c,r),gu(e)):Bs(e,t,r,gu(e))}catch(n){Bs(e,t,{then:function(){},status:`rejected`,reason:n},gu())}finally{D.p=a,o!==null&&s.types!==null&&(o.types=s.types),E.T=o}}function As(){}function js(e,t,n,r){if(e.tag!==5)throw Error(s(476));var i=Ms(e).queue;ks(e,i,t,ue,n===null?As:function(){return Ns(e),n(r)})}function Ms(e){var t=e.memoizedState;if(t!==null)return t;t={memoizedState:ue,baseState:ue,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:Wo,lastRenderedState:ue},next:null};var n={};return t.next={memoizedState:n,baseState:n,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:Wo,lastRenderedState:n},next:null},e.memoizedState=t,e=e.alternate,e!==null&&(e.memoizedState=t),t}function Ns(e){var t=Ms(e);t.next===null&&(t=e.alternate.memoizedState),Bs(e,t.next.queue,{},gu())}function Ps(){return ca(tp)}function Fs(){return zo().memoizedState}function Is(){return zo().memoizedState}function Ls(e){for(var t=e.return;t!==null;){switch(t.tag){case 24:case 3:var n=gu();e=Xa(n);var r=Za(t,e,n);r!==null&&(vu(r,t,n),Qa(r,t,n)),t={cache:ha()},e.payload=t;return}t=t.return}}function Rs(e,t,n){var r=gu();n={lane:r,revertLane:0,gesture:null,action:n,hasEagerState:!1,eagerState:null,next:null},Hs(e)?Us(t,n):(n=ui(e,t,n,r),n!==null&&(vu(n,e,r),Ws(n,t,r)))}function zs(e,t,n){Bs(e,t,n,gu())}function Bs(e,t,n,r){var i={lane:r,revertLane:0,gesture:null,action:n,hasEagerState:!1,eagerState:null,next:null};if(Hs(e))Us(t,i);else{var a=e.alternate;if(e.lanes===0&&(a===null||a.lanes===0)&&(a=t.lastRenderedReducer,a!==null))try{var o=t.lastRenderedState,s=a(o,n);if(i.hasEagerState=!0,i.eagerState=s,jr(s,o))return li(e,t,i,0),U===null&&ci(),!1}catch{}if(n=ui(e,t,i,r),n!==null)return vu(n,e,r),Ws(n,t,r),!0}return!1}function Vs(e,t,n,r){if(r={lane:2,revertLane:hd(),gesture:null,action:r,hasEagerState:!1,eagerState:null,next:null},Hs(e)){if(t)throw Error(s(479))}else t=ui(e,n,r,2),t!==null&&vu(t,e,2)}function Hs(e){var t=e.alternate;return e===P||t!==null&&t===P}function Us(e,t){Co=So=!0;var n=e.pending;n===null?t.next=t:(t.next=n.next,n.next=t),e.pending=t}function Ws(e,t,n){if(n&4194048){var r=t.lanes;r&=e.pendingLanes,n|=r,t.lanes=n,ut(e,n)}}var Gs={readContext:ca,use:Ho,useCallback:ko,useContext:ko,useEffect:ko,useImperativeHandle:ko,useLayoutEffect:ko,useInsertionEffect:ko,useMemo:ko,useReducer:ko,useRef:ko,useState:ko,useDebugValue:ko,useDeferredValue:ko,useTransition:ko,useSyncExternalStore:ko,useId:ko,useHostTransitionStatus:ko,useFormState:ko,useActionState:ko,useOptimistic:ko,useMemoCache:ko,useCacheRefresh:ko};Gs.useEffectEvent=ko;var Ks={readContext:ca,use:Ho,useCallback:function(e,t){return Ro().memoizedState=[e,t===void 0?null:t],e},useContext:ca,useEffect:_s,useImperativeHandle:function(e,t,n){n=n==null?null:n.concat([e]),gs(4194308,4,Ss.bind(null,t,e),n)},useLayoutEffect:function(e,t){return gs(4194308,4,e,t)},useInsertionEffect:function(e,t){gs(4,2,e,t)},useMemo:function(e,t){var n=Ro();t=t===void 0?null:t;var r=e();if(wo){qe(!0);try{e()}finally{qe(!1)}}return n.memoizedState=[r,t],r},useReducer:function(e,t,n){var r=Ro();if(n!==void 0){var i=n(t);if(wo){qe(!0);try{n(t)}finally{qe(!1)}}}else i=t;return r.memoizedState=r.baseState=i,e={pending:null,lanes:0,dispatch:null,lastRenderedReducer:e,lastRenderedState:i},r.queue=e,e=e.dispatch=Rs.bind(null,P,e),[r.memoizedState,e]},useRef:function(e){var t=Ro();return e={current:e},t.memoizedState=e},useState:function(e){e=es(e);var t=e.queue,n=zs.bind(null,P,t);return t.dispatch=n,[e.memoizedState,n]},useDebugValue:ws,useDeferredValue:function(e,t){return Ds(Ro(),e,t)},useTransition:function(){var e=es(!1);return e=ks.bind(null,P,e.queue,!0,!1),Ro().memoizedState=e,[!1,e]},useSyncExternalStore:function(e,t,n){var r=P,i=Ro();if(j){if(n===void 0)throw Error(s(407));n=n()}else{if(n=t(),U===null)throw Error(s(349));G&127||Yo(r,t,n)}i.memoizedState=n;var a={value:n,getSnapshot:t};return i.queue=a,_s(Zo.bind(null,r,a,e),[e]),r.flags|=2048,ms(9,{destroy:void 0},Xo.bind(null,r,a,n,t),null),n},useId:function(){var e=Ro(),t=U.identifierPrefix;if(j){var n=Fi,r=Pi;n=(r&~(1<<32-Je(r)-1)).toString(32)+n,t=`_`+t+`R_`+n,n=To++,0<n&&(t+=`H`+n.toString(32)),t+=`_`}else n=Oo++,t=`_`+t+`r_`+n.toString(32)+`_`;return e.memoizedState=t},useHostTransitionStatus:Ps,useFormState:ls,useActionState:ls,useOptimistic:function(e){var t=Ro();t.memoizedState=t.baseState=e;var n={pending:null,lanes:0,dispatch:null,lastRenderedReducer:null,lastRenderedState:null};return t.queue=n,t=Vs.bind(null,P,!0,n),n.dispatch=t,[e,t]},useMemoCache:Uo,useCacheRefresh:function(){return Ro().memoizedState=Ls.bind(null,P)},useEffectEvent:function(e){var t=Ro(),n={impl:e};return t.memoizedState=n,function(){if(H&2)throw Error(s(440));return n.impl.apply(void 0,arguments)}}},qs={readContext:ca,use:Ho,useCallback:Ts,useContext:ca,useEffect:vs,useImperativeHandle:Cs,useInsertionEffect:bs,useLayoutEffect:xs,useMemo:Es,useReducer:Go,useRef:hs,useState:function(){return Go(Wo)},useDebugValue:ws,useDeferredValue:function(e,t){return Os(zo(),bo.memoizedState,e,t)},useTransition:function(){var e=Go(Wo)[0],t=zo().memoizedState;return[typeof e==`boolean`?e:Vo(e),t]},useSyncExternalStore:Jo,useId:Fs,useHostTransitionStatus:Ps,useFormState:us,useActionState:us,useOptimistic:function(e,t){return ts(zo(),bo,e,t)},useMemoCache:Uo,useCacheRefresh:Is};qs.useEffectEvent=ys;var Js={readContext:ca,use:Ho,useCallback:Ts,useContext:ca,useEffect:vs,useImperativeHandle:Cs,useInsertionEffect:bs,useLayoutEffect:xs,useMemo:Es,useReducer:qo,useRef:hs,useState:function(){return qo(Wo)},useDebugValue:ws,useDeferredValue:function(e,t){var n=zo();return bo===null?Ds(n,e,t):Os(n,bo.memoizedState,e,t)},useTransition:function(){var e=qo(Wo)[0],t=zo().memoizedState;return[typeof e==`boolean`?e:Vo(e),t]},useSyncExternalStore:Jo,useId:Fs,useHostTransitionStatus:Ps,useFormState:ps,useActionState:ps,useOptimistic:function(e,t){var n=zo();return bo===null?(n.baseState=e,[e,n.queue.dispatch]):ts(n,bo,e,t)},useMemoCache:Uo,useCacheRefresh:Is};Js.useEffectEvent=ys;function Ys(e,t,n,r){t=e.memoizedState,n=n(r,t),n=n==null?t:h({},t,n),e.memoizedState=n,e.lanes===0&&(e.updateQueue.baseState=n)}var Xs={enqueueSetState:function(e,t,n){e=e._reactInternals;var r=gu(),i=Xa(r);i.payload=t,n!=null&&(i.callback=n),t=Za(e,i,r),t!==null&&(vu(t,e,r),Qa(t,e,r))},enqueueReplaceState:function(e,t,n){e=e._reactInternals;var r=gu(),i=Xa(r);i.tag=1,i.payload=t,n!=null&&(i.callback=n),t=Za(e,i,r),t!==null&&(vu(t,e,r),Qa(t,e,r))},enqueueForceUpdate:function(e,t){e=e._reactInternals;var n=gu(),r=Xa(n);r.tag=2,t!=null&&(r.callback=t),t=Za(e,r,n),t!==null&&(vu(t,e,n),Qa(t,e,n))}};function Zs(e,t,n,r,i,a,o){return e=e.stateNode,typeof e.shouldComponentUpdate==`function`?e.shouldComponentUpdate(r,a,o):t.prototype&&t.prototype.isPureReactComponent?!Mr(n,r)||!Mr(i,a):!0}function Qs(e,t,n,r){e=t.state,typeof t.componentWillReceiveProps==`function`&&t.componentWillReceiveProps(n,r),typeof t.UNSAFE_componentWillReceiveProps==`function`&&t.UNSAFE_componentWillReceiveProps(n,r),t.state!==e&&Xs.enqueueReplaceState(t,t.state,null)}function $s(e,t){var n=t;if(`ref`in t)for(var r in n={},t)r!==`ref`&&(n[r]=t[r]);if(e=e.defaultProps)for(var i in n===t&&(n=h({},n)),e)n[i]===void 0&&(n[i]=e[i]);return n}function ec(e){ii(e)}function tc(e){console.error(e)}function nc(e){ii(e)}function rc(e,t){try{var n=e.onUncaughtError;n(t.value,{componentStack:t.stack})}catch(e){setTimeout(function(){throw e})}}function ic(e,t,n){try{var r=e.onCaughtError;r(n.value,{componentStack:n.stack,errorBoundary:t.tag===1?t.stateNode:null})}catch(e){setTimeout(function(){throw e})}}function ac(e,t,n){return n=Xa(n),n.tag=3,n.payload={element:null},n.callback=function(){rc(e,t)},n}function oc(e){return e=Xa(e),e.tag=3,e}function sc(e,t,n,r){var i=n.type.getDerivedStateFromError;if(typeof i==`function`){var a=r.value;e.payload=function(){return i(a)},e.callback=function(){ic(t,n,r)}}var o=n.stateNode;o!==null&&typeof o.componentDidCatch==`function`&&(e.callback=function(){ic(t,n,r),typeof i!=`function`&&(q===null?q=new Set([this]):q.add(this));var e=r.stack;this.componentDidCatch(r.value,{componentStack:e===null?``:e})})}function cc(e,t,n,r,i){if(n.flags|=32768,typeof r==`object`&&r&&typeof r.then==`function`){if(t=n.alternate,t!==null&&aa(t,n,i,!0),n=uo.current,n!==null){switch(n.tag){case 31:case 13:return fo===null?Au():n.alternate===null&&Jl===0&&(Jl=3),n.flags&=-257,n.flags|=65536,n.lanes=i,r===Ma?n.flags|=16384:(t=n.updateQueue,t===null?n.updateQueue=new Set([r]):t.add(r),Yu(e,r,i)),!1;case 22:return n.flags|=65536,r===Ma?n.flags|=16384:(t=n.updateQueue,t===null?(t={transitions:null,markerInstances:null,retryQueue:new Set([r])},n.updateQueue=t):(n=t.retryQueue,n===null?t.retryQueue=new Set([r]):n.add(r)),Yu(e,r,i)),!1}throw Error(s(435,n.tag))}return Yu(e,r,i),Au(),!1}if(j)return t=uo.current,t===null?(r!==Gi&&(t=Error(s(423),{cause:r}),Qi(Ei(t,n))),e=e.current.alternate,e.flags|=65536,i&=-i,e.lanes|=i,r=Ei(r,n),i=ac(e.stateNode,r,i),$a(e,i),Jl!==4&&(Jl=2)):(!(t.flags&65536)&&(t.flags|=256),t.flags|=65536,t.lanes=i,r!==Gi&&(e=Error(s(422),{cause:r}),Qi(Ei(e,n)))),!1;var a=Error(s(520),{cause:r});if(a=Ei(a,n),eu===null?eu=[a]:eu.push(a),Jl!==4&&(Jl=2),t===null)return!0;r=Ei(r,n),n=t;do{switch(n.tag){case 3:return n.flags|=65536,e=i&-i,n.lanes|=e,e=ac(n.stateNode,r,e),$a(n,e),!1;case 1:if(t=n.type,a=n.stateNode,!(n.flags&128)&&(typeof t.getDerivedStateFromError==`function`||a!==null&&typeof a.componentDidCatch==`function`&&(q===null||!q.has(a))))return n.flags|=65536,i&=-i,n.lanes|=i,i=oc(i),sc(i,e,n,r),$a(n,i),!1}n=n.return}while(n!==null);return!1}var lc=Error(s(461)),uc=!1;function dc(e,t,n,r){t.child=e===null?Ka(t,null,n,r):Ga(t,e.child,n,r)}function fc(e,t,n,r,i){n=n.render;var a=t.ref;if(`ref`in r){var o={};for(var s in r)s!==`ref`&&(o[s]=r[s])}else o=r;return sa(t),r=jo(e,t,n,o,a,i),s=Fo(),e!==null&&!uc?(Io(e,t,i),Ic(e,t,i)):(j&&s&&Ri(t),t.flags|=1,dc(e,t,r,i),t.child)}function pc(e,t,n,r,i){if(e===null){var a=n.type;return typeof a==`function`&&!_i(a)&&a.defaultProps===void 0&&n.compare===null?(t.tag=15,t.type=a,mc(e,t,a,r,i)):(e=bi(n.type,null,r,t,t.mode,i),e.ref=t.ref,e.return=t,t.child=e)}if(a=e.child,!Lc(e,i)){var o=a.memoizedProps;if(n=n.compare,n=n===null?Mr:n,n(o,r)&&e.ref===t.ref)return Ic(e,t,i)}return t.flags|=1,e=vi(a,r),e.ref=t.ref,e.return=t,t.child=e}function mc(e,t,n,r,i){if(e!==null){var a=e.memoizedProps;if(Mr(a,r)&&e.ref===t.ref)if(uc=!1,t.pendingProps=r=a,Lc(e,i))e.flags&131072&&(uc=!0);else return t.lanes=e.lanes,Ic(e,t,i)}return Sc(e,t,n,r,i)}function hc(e,t,n,r){var i=r.children,a=e===null?null:e.memoizedState;if(e===null&&t.stateNode===null&&(t.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),r.mode===`hidden`){if(t.flags&128){if(a=a===null?n:a.baseLanes|n,e!==null){for(r=t.child=e.child,i=0;r!==null;)i=i|r.lanes|r.childLanes,r=r.sibling;r=i&~a}else r=0,t.child=null;return _c(e,t,a,n,r)}if(n&536870912)t.memoizedState={baseLanes:0,cachePool:null},e!==null&&Da(t,a===null?null:a.cachePool),a===null?co():so(t,a),mo(t);else return r=t.lanes=536870912,_c(e,t,a===null?n:a.baseLanes|n,n,r)}else a===null?(e!==null&&Da(t,null),co(),ho(t)):(Da(t,a.cachePool),so(t,a),ho(t),t.memoizedState=null);return dc(e,t,i,n),t.child}function gc(e,t){return e!==null&&e.tag===22||t.stateNode!==null||(t.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),t.sibling}function _c(e,t,n,r,i){var a=Ea();return a=a===null?null:{parent:ma._currentValue,pool:a},t.memoizedState={baseLanes:n,cachePool:a},e!==null&&Da(t,null),co(),mo(t),e!==null&&aa(e,t,r,!0),t.childLanes=i,null}function vc(e,t){return t=jc({mode:t.mode,children:t.children},e.mode),t.ref=e.ref,e.child=t,t.return=e,t}function yc(e,t,n){return Ga(t,e.child,null,n),e=vc(t,t.pendingProps),e.flags|=2,go(t),t.memoizedState=null,e}function bc(e,t,n){var r=t.pendingProps,i=(t.flags&128)!=0;if(t.flags&=-129,e===null){if(j){if(r.mode===`hidden`)return e=vc(t,r),t.lanes=536870912,gc(null,e);if(N(t),(e=Hi)?(e=sf(e,Wi),e=e!==null&&e.data===`&`?e:null,e!==null&&(t.memoizedState={dehydrated:e,treeContext:Ni===null?null:{id:Pi,overflow:Fi},retryLane:536870912,hydrationErrors:null},n=Ci(e),n.return=t,t.child=n,Vi=t,Hi=null)):e=null,e===null)throw Ki(t);return t.lanes=536870912,null}return vc(t,r)}var a=e.memoizedState;if(a!==null){var o=a.dehydrated;if(N(t),i)if(t.flags&256)t.flags&=-257,t=yc(e,t,n);else if(t.memoizedState!==null)t.child=e.child,t.flags|=128,t=null;else throw Error(s(558));else if(uc||aa(e,t,n,!1),i=(n&e.childLanes)!==0,uc||i){if(r=U,r!==null&&(o=dt(r,n),o!==0&&o!==a.retryLane))throw a.retryLane=o,di(e,o),vu(r,e,o),lc;Au(),t=yc(e,t,n)}else e=a.treeContext,Hi=df(o.nextSibling),Vi=t,j=!0,Ui=null,Wi=!1,e!==null&&Bi(t,e),t=vc(t,r),t.flags|=4096;return t}return e=vi(e.child,{mode:r.mode,children:r.children}),e.ref=t.ref,t.child=e,e.return=t,e}function xc(e,t){var n=t.ref;if(n===null)e!==null&&e.ref!==null&&(t.flags|=4194816);else{if(typeof n!=`function`&&typeof n!=`object`)throw Error(s(284));(e===null||e.ref!==n)&&(t.flags|=4194816)}}function Sc(e,t,n,r,i){return sa(t),n=jo(e,t,n,r,void 0,i),r=Fo(),e!==null&&!uc?(Io(e,t,i),Ic(e,t,i)):(j&&r&&Ri(t),t.flags|=1,dc(e,t,n,i),t.child)}function Cc(e,t,n,r,i,a){return sa(t),t.updateQueue=null,n=No(t,r,n,i),Mo(e),r=Fo(),e!==null&&!uc?(Io(e,t,a),Ic(e,t,a)):(j&&r&&Ri(t),t.flags|=1,dc(e,t,n,a),t.child)}function wc(e,t,n,r,i){if(sa(t),t.stateNode===null){var a=mi,o=n.contextType;typeof o==`object`&&o&&(a=ca(o)),a=new n(r,a),t.memoizedState=a.state!==null&&a.state!==void 0?a.state:null,a.updater=Xs,t.stateNode=a,a._reactInternals=t,a=t.stateNode,a.props=r,a.state=t.memoizedState,a.refs={},Ja(t),o=n.contextType,a.context=typeof o==`object`&&o?ca(o):mi,a.state=t.memoizedState,o=n.getDerivedStateFromProps,typeof o==`function`&&(Ys(t,n,o,r),a.state=t.memoizedState),typeof n.getDerivedStateFromProps==`function`||typeof a.getSnapshotBeforeUpdate==`function`||typeof a.UNSAFE_componentWillMount!=`function`&&typeof a.componentWillMount!=`function`||(o=a.state,typeof a.componentWillMount==`function`&&a.componentWillMount(),typeof a.UNSAFE_componentWillMount==`function`&&a.UNSAFE_componentWillMount(),o!==a.state&&Xs.enqueueReplaceState(a,a.state,null),no(t,r,a,i),to(),a.state=t.memoizedState),typeof a.componentDidMount==`function`&&(t.flags|=4194308),r=!0}else if(e===null){a=t.stateNode;var s=t.memoizedProps,c=$s(n,s);a.props=c;var l=a.context,u=n.contextType;o=mi,typeof u==`object`&&u&&(o=ca(u));var d=n.getDerivedStateFromProps;u=typeof d==`function`||typeof a.getSnapshotBeforeUpdate==`function`,s=t.pendingProps!==s,u||typeof a.UNSAFE_componentWillReceiveProps!=`function`&&typeof a.componentWillReceiveProps!=`function`||(s||l!==o)&&Qs(t,a,r,o),qa=!1;var f=t.memoizedState;a.state=f,no(t,r,a,i),to(),l=t.memoizedState,s||f!==l||qa?(typeof d==`function`&&(Ys(t,n,d,r),l=t.memoizedState),(c=qa||Zs(t,n,c,r,f,l,o))?(u||typeof a.UNSAFE_componentWillMount!=`function`&&typeof a.componentWillMount!=`function`||(typeof a.componentWillMount==`function`&&a.componentWillMount(),typeof a.UNSAFE_componentWillMount==`function`&&a.UNSAFE_componentWillMount()),typeof a.componentDidMount==`function`&&(t.flags|=4194308)):(typeof a.componentDidMount==`function`&&(t.flags|=4194308),t.memoizedProps=r,t.memoizedState=l),a.props=r,a.state=l,a.context=o,r=c):(typeof a.componentDidMount==`function`&&(t.flags|=4194308),r=!1)}else{a=t.stateNode,Ya(e,t),o=t.memoizedProps,u=$s(n,o),a.props=u,d=t.pendingProps,f=a.context,l=n.contextType,c=mi,typeof l==`object`&&l&&(c=ca(l)),s=n.getDerivedStateFromProps,(l=typeof s==`function`||typeof a.getSnapshotBeforeUpdate==`function`)||typeof a.UNSAFE_componentWillReceiveProps!=`function`&&typeof a.componentWillReceiveProps!=`function`||(o!==d||f!==c)&&Qs(t,a,r,c),qa=!1,f=t.memoizedState,a.state=f,no(t,r,a,i),to();var p=t.memoizedState;o!==d||f!==p||qa||e!==null&&e.dependencies!==null&&oa(e.dependencies)?(typeof s==`function`&&(Ys(t,n,s,r),p=t.memoizedState),(u=qa||Zs(t,n,u,r,f,p,c)||e!==null&&e.dependencies!==null&&oa(e.dependencies))?(l||typeof a.UNSAFE_componentWillUpdate!=`function`&&typeof a.componentWillUpdate!=`function`||(typeof a.componentWillUpdate==`function`&&a.componentWillUpdate(r,p,c),typeof a.UNSAFE_componentWillUpdate==`function`&&a.UNSAFE_componentWillUpdate(r,p,c)),typeof a.componentDidUpdate==`function`&&(t.flags|=4),typeof a.getSnapshotBeforeUpdate==`function`&&(t.flags|=1024)):(typeof a.componentDidUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=4),typeof a.getSnapshotBeforeUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=1024),t.memoizedProps=r,t.memoizedState=p),a.props=r,a.state=p,a.context=c,r=u):(typeof a.componentDidUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=4),typeof a.getSnapshotBeforeUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=1024),r=!1)}return a=r,xc(e,t),r=(t.flags&128)!=0,a||r?(a=t.stateNode,n=r&&typeof n.getDerivedStateFromError!=`function`?null:a.render(),t.flags|=1,e!==null&&r?(t.child=Ga(t,e.child,null,i),t.child=Ga(t,null,n,i)):dc(e,t,n,i),t.memoizedState=a.state,e=t.child):e=Ic(e,t,i),e}function Tc(e,t,n,r){return Xi(),t.flags|=256,dc(e,t,n,r),t.child}var Ec={dehydrated:null,treeContext:null,retryLane:0,hydrationErrors:null};function Dc(e){return{baseLanes:e,cachePool:Oa()}}function Oc(e,t,n){return e=e===null?0:e.childLanes&~n,t&&(e|=Ql),e}function kc(e,t,n){var r=t.pendingProps,i=!1,a=(t.flags&128)!=0,o;if((o=a)||(o=e!==null&&e.memoizedState===null?!1:(_o.current&2)!=0),o&&(i=!0,t.flags&=-129),o=(t.flags&32)!=0,t.flags&=-33,e===null){if(j){if(i?po(t):ho(t),(e=Hi)?(e=sf(e,Wi),e=e!==null&&e.data!==`&`?e:null,e!==null&&(t.memoizedState={dehydrated:e,treeContext:Ni===null?null:{id:Pi,overflow:Fi},retryLane:536870912,hydrationErrors:null},n=Ci(e),n.return=t,t.child=n,Vi=t,Hi=null)):e=null,e===null)throw Ki(t);return lf(e)?t.lanes=32:t.lanes=536870912,null}var c=r.children;return r=r.fallback,i?(ho(t),i=t.mode,c=jc({mode:`hidden`,children:c},i),r=xi(r,i,n,null),c.return=t,r.return=t,c.sibling=r,t.child=c,r=t.child,r.memoizedState=Dc(n),r.childLanes=Oc(e,o,n),t.memoizedState=Ec,gc(null,r)):(po(t),Ac(t,c))}var l=e.memoizedState;if(l!==null&&(c=l.dehydrated,c!==null)){if(a)t.flags&256?(po(t),t.flags&=-257,t=Mc(e,t,n)):t.memoizedState===null?(ho(t),c=r.fallback,i=t.mode,r=jc({mode:`visible`,children:r.children},i),c=xi(c,i,n,null),c.flags|=2,r.return=t,c.return=t,r.sibling=c,t.child=r,Ga(t,e.child,null,n),r=t.child,r.memoizedState=Dc(n),r.childLanes=Oc(e,o,n),t.memoizedState=Ec,t=gc(null,r)):(ho(t),t.child=e.child,t.flags|=128,t=null);else if(po(t),lf(c)){if(o=c.nextSibling&&c.nextSibling.dataset,o)var u=o.dgst;o=u,r=Error(s(419)),r.stack=``,r.digest=o,Qi({value:r,source:null,stack:null}),t=Mc(e,t,n)}else if(uc||aa(e,t,n,!1),o=(n&e.childLanes)!==0,uc||o){if(o=U,o!==null&&(r=dt(o,n),r!==0&&r!==l.retryLane))throw l.retryLane=r,di(e,r),vu(o,e,r),lc;cf(c)||Au(),t=Mc(e,t,n)}else cf(c)?(t.flags|=192,t.child=e.child,t=null):(e=l.treeContext,Hi=df(c.nextSibling),Vi=t,j=!0,Ui=null,Wi=!1,e!==null&&Bi(t,e),t=Ac(t,r.children),t.flags|=4096);return t}return i?(ho(t),c=r.fallback,i=t.mode,l=e.child,u=l.sibling,r=vi(l,{mode:`hidden`,children:r.children}),r.subtreeFlags=l.subtreeFlags&65011712,u===null?(c=xi(c,i,n,null),c.flags|=2):c=vi(u,c),c.return=t,r.return=t,r.sibling=c,t.child=r,gc(null,r),r=t.child,c=e.child.memoizedState,c===null?c=Dc(n):(i=c.cachePool,i===null?i=Oa():(l=ma._currentValue,i=i.parent===l?i:{parent:l,pool:l}),c={baseLanes:c.baseLanes|n,cachePool:i}),r.memoizedState=c,r.childLanes=Oc(e,o,n),t.memoizedState=Ec,gc(e.child,r)):(po(t),n=e.child,e=n.sibling,n=vi(n,{mode:`visible`,children:r.children}),n.return=t,n.sibling=null,e!==null&&(o=t.deletions,o===null?(t.deletions=[e],t.flags|=16):o.push(e)),t.child=n,t.memoizedState=null,n)}function Ac(e,t){return t=jc({mode:`visible`,children:t},e.mode),t.return=e,e.child=t}function jc(e,t){return e=gi(22,e,null,t),e.lanes=0,e}function Mc(e,t,n){return Ga(t,e.child,null,n),e=Ac(t,t.pendingProps.children),e.flags|=2,t.memoizedState=null,e}function Nc(e,t,n){e.lanes|=t;var r=e.alternate;r!==null&&(r.lanes|=t),ia(e.return,t,n)}function Pc(e,t,n,r,i,a){var o=e.memoizedState;o===null?e.memoizedState={isBackwards:t,rendering:null,renderingStartTime:0,last:r,tail:n,tailMode:i,treeForkCount:a}:(o.isBackwards=t,o.rendering=null,o.renderingStartTime=0,o.last=r,o.tail=n,o.tailMode=i,o.treeForkCount=a)}function Fc(e,t,n){var r=t.pendingProps,i=r.revealOrder,a=r.tail;r=r.children;var o=_o.current,s=(o&2)!=0;if(s?(o=o&1|2,t.flags|=128):o&=1,he(_o,o),dc(e,t,r,n),r=j?Ai:0,!s&&e!==null&&e.flags&128)a:for(e=t.child;e!==null;){if(e.tag===13)e.memoizedState!==null&&Nc(e,n,t);else if(e.tag===19)Nc(e,n,t);else if(e.child!==null){e.child.return=e,e=e.child;continue}if(e===t)break a;for(;e.sibling===null;){if(e.return===null||e.return===t)break a;e=e.return}e.sibling.return=e.return,e=e.sibling}switch(i){case`forwards`:for(n=t.child,i=null;n!==null;)e=n.alternate,e!==null&&vo(e)===null&&(i=n),n=n.sibling;n=i,n===null?(i=t.child,t.child=null):(i=n.sibling,n.sibling=null),Pc(t,!1,i,n,a,r);break;case`backwards`:case`unstable_legacy-backwards`:for(n=null,i=t.child,t.child=null;i!==null;){if(e=i.alternate,e!==null&&vo(e)===null){t.child=i;break}e=i.sibling,i.sibling=n,n=i,i=e}Pc(t,!0,n,null,a,r);break;case`together`:Pc(t,!1,null,null,void 0,r);break;default:t.memoizedState=null}return t.child}function Ic(e,t,n){if(e!==null&&(t.dependencies=e.dependencies),Yl|=t.lanes,(n&t.childLanes)===0)if(e!==null){if(aa(e,t,n,!1),(n&t.childLanes)===0)return null}else return null;if(e!==null&&t.child!==e.child)throw Error(s(153));if(t.child!==null){for(e=t.child,n=vi(e,e.pendingProps),t.child=n,n.return=t;e.sibling!==null;)e=e.sibling,n=n.sibling=vi(e,e.pendingProps),n.return=t;n.sibling=null}return t.child}function Lc(e,t){return(e.lanes&t)===0?(e=e.dependencies,!!(e!==null&&oa(e))):!0}function Rc(e,t,n){switch(t.tag){case 3:be(t,t.stateNode.containerInfo),na(t,ma,e.memoizedState.cache),Xi();break;case 27:case 5:Se(t);break;case 4:be(t,t.stateNode.containerInfo);break;case 10:na(t,t.type,t.memoizedProps.value);break;case 31:if(t.memoizedState!==null)return t.flags|=128,N(t),null;break;case 13:var r=t.memoizedState;if(r!==null)return r.dehydrated===null?(n&t.child.childLanes)===0?(po(t),e=Ic(e,t,n),e===null?null:e.sibling):kc(e,t,n):(po(t),t.flags|=128,null);po(t);break;case 19:var i=(e.flags&128)!=0;if(r=(n&t.childLanes)!==0,r||=(aa(e,t,n,!1),(n&t.childLanes)!==0),i){if(r)return Fc(e,t,n);t.flags|=128}if(i=t.memoizedState,i!==null&&(i.rendering=null,i.tail=null,i.lastEffect=null),he(_o,_o.current),r)break;return null;case 22:return t.lanes=0,hc(e,t,n,t.pendingProps);case 24:na(t,ma,e.memoizedState.cache)}return Ic(e,t,n)}function L(e,t,n){if(e!==null)if(e.memoizedProps!==t.pendingProps)uc=!0;else{if(!Lc(e,n)&&!(t.flags&128))return uc=!1,Rc(e,t,n);uc=!!(e.flags&131072)}else uc=!1,j&&t.flags&1048576&&Li(t,Ai,t.index);switch(t.lanes=0,t.tag){case 16:a:{var r=t.pendingProps;if(e=Fa(t.elementType),t.type=e,typeof e==`function`)_i(e)?(r=$s(e,r),t.tag=1,t=wc(null,t,e,r,n)):(t.tag=0,t=Sc(null,t,e,r,n));else{if(e!=null){var i=e.$$typeof;if(i===C){t.tag=11,t=fc(null,t,e,r,n);break a}else if(i===w){t.tag=14,t=pc(null,t,e,r,n);break a}}throw t=ce(e)||e,Error(s(306,t,``))}}return t;case 0:return Sc(e,t,t.type,t.pendingProps,n);case 1:return r=t.type,i=$s(r,t.pendingProps),wc(e,t,r,i,n);case 3:a:{if(be(t,t.stateNode.containerInfo),e===null)throw Error(s(387));r=t.pendingProps;var a=t.memoizedState;i=a.element,Ya(e,t),no(t,r,null,n);var o=t.memoizedState;if(r=o.cache,na(t,ma,r),r!==a.cache&&M(t,[ma],n,!0),to(),r=o.element,a.isDehydrated)if(a={element:r,isDehydrated:!1,cache:o.cache},t.updateQueue.baseState=a,t.memoizedState=a,t.flags&256){t=Tc(e,t,r,n);break a}else if(r!==i){i=Ei(Error(s(424)),t),Qi(i),t=Tc(e,t,r,n);break a}else{switch(e=t.stateNode.containerInfo,e.nodeType){case 9:e=e.body;break;default:e=e.nodeName===`HTML`?e.ownerDocument.body:e}for(Hi=df(e.firstChild),Vi=t,j=!0,Ui=null,Wi=!0,n=Ka(t,null,r,n),t.child=n;n;)n.flags=n.flags&-3|4096,n=n.sibling}else{if(Xi(),r===i){t=Ic(e,t,n);break a}dc(e,t,r,n)}t=t.child}return t;case 26:return xc(e,t),e===null?(n=Mf(t.type,null,t.pendingProps,null))?t.memoizedState=n:j||(n=t.type,e=t.pendingProps,r=Wd(ve.current).createElement(n),r[_t]=t,r[vt]=e,Rd(r,n,e),At(r),t.stateNode=r):t.memoizedState=Mf(t.type,e.memoizedProps,t.pendingProps,e.memoizedState),null;case 27:return Se(t),e===null&&j&&(r=t.stateNode=hf(t.type,t.pendingProps,ve.current),Vi=t,Wi=!0,i=Hi,tf(t.type)?(ff=i,Hi=df(r.firstChild)):Hi=i),dc(e,t,t.pendingProps.children,n),xc(e,t),e===null&&(t.flags|=4194304),t.child;case 5:return e===null&&j&&((i=r=Hi)&&(r=X(r,t.type,t.pendingProps,Wi),r===null?i=!1:(t.stateNode=r,Vi=t,Hi=df(r.firstChild),Wi=!1,i=!0)),i||Ki(t)),Se(t),i=t.type,a=t.pendingProps,o=e===null?null:e.memoizedProps,r=a.children,qd(i,a)?r=null:o!==null&&qd(i,o)&&(t.flags|=32),t.memoizedState!==null&&(i=jo(e,t,Po,null,null,n),tp._currentValue=i),xc(e,t),dc(e,t,r,n),t.child;case 6:return e===null&&j&&((e=n=Hi)&&(n=of(n,t.pendingProps,Wi),n===null?e=!1:(t.stateNode=n,Vi=t,Hi=null,e=!0)),e||Ki(t)),null;case 13:return kc(e,t,n);case 4:return be(t,t.stateNode.containerInfo),r=t.pendingProps,e===null?t.child=Ga(t,null,r,n):dc(e,t,r,n),t.child;case 11:return fc(e,t,t.type,t.pendingProps,n);case 7:return dc(e,t,t.pendingProps,n),t.child;case 8:return dc(e,t,t.pendingProps.children,n),t.child;case 12:return dc(e,t,t.pendingProps.children,n),t.child;case 10:return r=t.pendingProps,na(t,t.type,r.value),dc(e,t,r.children,n),t.child;case 9:return i=t.type._context,r=t.pendingProps.children,sa(t),i=ca(i),r=r(i),t.flags|=1,dc(e,t,r,n),t.child;case 14:return pc(e,t,t.type,t.pendingProps,n);case 15:return mc(e,t,t.type,t.pendingProps,n);case 19:return Fc(e,t,n);case 31:return bc(e,t,n);case 22:return hc(e,t,n,t.pendingProps);case 24:return sa(t),r=ca(ma),e===null?(i=Ea(),i===null&&(i=U,a=ha(),i.pooledCache=a,a.refCount++,a!==null&&(i.pooledCacheLanes|=n),i=a),t.memoizedState={parent:r,cache:i},Ja(t),na(t,ma,i)):((e.lanes&n)!==0&&(Ya(e,t),no(t,null,null,n),to()),i=e.memoizedState,a=t.memoizedState,i.parent===r?(r=a.cache,na(t,ma,r),r!==i.cache&&M(t,[ma],n,!0)):(i={parent:r,cache:r},t.memoizedState=i,t.lanes===0&&(t.memoizedState=t.updateQueue.baseState=i),na(t,ma,r))),dc(e,t,t.pendingProps.children,n),t.child;case 29:throw t.pendingProps}throw Error(s(156,t.tag))}function zc(e){e.flags|=4}function Bc(e,t,n,r,i){if((t=(e.mode&32)!=0)&&(t=!1),t){if(e.flags|=16777216,(i&335544128)===i)if(e.stateNode.complete)e.flags|=8192;else if(Du())e.flags|=8192;else throw Ia=Ma,Aa}else e.flags&=-16777217}function Vc(e,t){if(t.type!==`stylesheet`||t.state.loading&4)e.flags&=-16777217;else if(e.flags|=16777216,!qf(t))if(Du())e.flags|=8192;else throw Ia=Ma,Aa}function Hc(e,t){t!==null&&(e.flags|=4),e.flags&16384&&(t=e.tag===22?536870912:at(),e.lanes|=t,$l|=t)}function Uc(e,t){if(!j)switch(e.tailMode){case`hidden`:t=e.tail;for(var n=null;t!==null;)t.alternate!==null&&(n=t),t=t.sibling;n===null?e.tail=null:n.sibling=null;break;case`collapsed`:n=e.tail;for(var r=null;n!==null;)n.alternate!==null&&(r=n),n=n.sibling;r===null?t||e.tail===null?e.tail=null:e.tail.sibling=null:r.sibling=null}}function Wc(e){var t=e.alternate!==null&&e.alternate.child===e.child,n=0,r=0;if(t)for(var i=e.child;i!==null;)n|=i.lanes|i.childLanes,r|=i.subtreeFlags&65011712,r|=i.flags&65011712,i.return=e,i=i.sibling;else for(i=e.child;i!==null;)n|=i.lanes|i.childLanes,r|=i.subtreeFlags,r|=i.flags,i.return=e,i=i.sibling;return e.subtreeFlags|=r,e.childLanes=n,t}function Gc(e,t,n){var r=t.pendingProps;switch(zi(t),t.tag){case 16:case 15:case 0:case 11:case 7:case 8:case 12:case 9:case 14:return Wc(t),null;case 1:return Wc(t),null;case 3:return n=t.stateNode,r=null,e!==null&&(r=e.memoizedState.cache),t.memoizedState.cache!==r&&(t.flags|=2048),ra(ma),xe(),n.pendingContext&&(n.context=n.pendingContext,n.pendingContext=null),(e===null||e.child===null)&&(Yi(t)?zc(t):e===null||e.memoizedState.isDehydrated&&!(t.flags&256)||(t.flags|=1024,Zi())),Wc(t),null;case 26:var i=t.type,a=t.memoizedState;return e===null?(zc(t),a===null?(Wc(t),Bc(t,i,null,r,n)):(Wc(t),Vc(t,a))):a?a===e.memoizedState?(Wc(t),t.flags&=-16777217):(zc(t),Wc(t),Vc(t,a)):(e=e.memoizedProps,e!==r&&zc(t),Wc(t),Bc(t,i,e,r,n)),null;case 27:if(Ce(t),n=ve.current,i=t.type,e!==null&&t.stateNode!=null)e.memoizedProps!==r&&zc(t);else{if(!r){if(t.stateNode===null)throw Error(s(166));return Wc(t),null}e=ge.current,Yi(t)?qi(t,e):(e=hf(i,r,n),t.stateNode=e,zc(t))}return Wc(t),null;case 5:if(Ce(t),i=t.type,e!==null&&t.stateNode!=null)e.memoizedProps!==r&&zc(t);else{if(!r){if(t.stateNode===null)throw Error(s(166));return Wc(t),null}if(a=ge.current,Yi(t))qi(t,a);else{var o=Wd(ve.current);switch(a){case 1:a=o.createElementNS(`http://www.w3.org/2000/svg`,i);break;case 2:a=o.createElementNS(`http://www.w3.org/1998/Math/MathML`,i);break;default:switch(i){case`svg`:a=o.createElementNS(`http://www.w3.org/2000/svg`,i);break;case`math`:a=o.createElementNS(`http://www.w3.org/1998/Math/MathML`,i);break;case`script`:a=o.createElement(`div`),a.innerHTML=`<script><\/script>`,a=a.removeChild(a.firstChild);break;case`select`:a=typeof r.is==`string`?o.createElement(`select`,{is:r.is}):o.createElement(`select`),r.multiple?a.multiple=!0:r.size&&(a.size=r.size);break;default:a=typeof r.is==`string`?o.createElement(i,{is:r.is}):o.createElement(i)}}a[_t]=t,a[vt]=r;a:for(o=t.child;o!==null;){if(o.tag===5||o.tag===6)a.appendChild(o.stateNode);else if(o.tag!==4&&o.tag!==27&&o.child!==null){o.child.return=o,o=o.child;continue}if(o===t)break a;for(;o.sibling===null;){if(o.return===null||o.return===t)break a;o=o.return}o.sibling.return=o.return,o=o.sibling}t.stateNode=a;a:switch(Rd(a,i,r),i){case`button`:case`input`:case`select`:case`textarea`:r=!!r.autoFocus;break a;case`img`:r=!0;break a;default:r=!1}r&&zc(t)}}return Wc(t),Bc(t,t.type,e===null?null:e.memoizedProps,t.pendingProps,n),null;case 6:if(e&&t.stateNode!=null)e.memoizedProps!==r&&zc(t);else{if(typeof r!=`string`&&t.stateNode===null)throw Error(s(166));if(e=ve.current,Yi(t)){if(e=t.stateNode,n=t.memoizedProps,r=null,i=Vi,i!==null)switch(i.tag){case 27:case 5:r=i.memoizedProps}e[_t]=t,e=!!(e.nodeValue===n||r!==null&&!0===r.suppressHydrationWarning||Fd(e.nodeValue,n)),e||Ki(t,!0)}else e=Wd(e).createTextNode(r),e[_t]=t,t.stateNode=e}return Wc(t),null;case 31:if(n=t.memoizedState,e===null||e.memoizedState!==null){if(r=Yi(t),n!==null){if(e===null){if(!r)throw Error(s(318));if(e=t.memoizedState,e=e===null?null:e.dehydrated,!e)throw Error(s(557));e[_t]=t}else Xi(),!(t.flags&128)&&(t.memoizedState=null),t.flags|=4;Wc(t),e=!1}else n=Zi(),e!==null&&e.memoizedState!==null&&(e.memoizedState.hydrationErrors=n),e=!0;if(!e)return t.flags&256?(go(t),t):(go(t),null);if(t.flags&128)throw Error(s(558))}return Wc(t),null;case 13:if(r=t.memoizedState,e===null||e.memoizedState!==null&&e.memoizedState.dehydrated!==null){if(i=Yi(t),r!==null&&r.dehydrated!==null){if(e===null){if(!i)throw Error(s(318));if(i=t.memoizedState,i=i===null?null:i.dehydrated,!i)throw Error(s(317));i[_t]=t}else Xi(),!(t.flags&128)&&(t.memoizedState=null),t.flags|=4;Wc(t),i=!1}else i=Zi(),e!==null&&e.memoizedState!==null&&(e.memoizedState.hydrationErrors=i),i=!0;if(!i)return t.flags&256?(go(t),t):(go(t),null)}return go(t),t.flags&128?(t.lanes=n,t):(n=r!==null,e=e!==null&&e.memoizedState!==null,n&&(r=t.child,i=null,r.alternate!==null&&r.alternate.memoizedState!==null&&r.alternate.memoizedState.cachePool!==null&&(i=r.alternate.memoizedState.cachePool.pool),a=null,r.memoizedState!==null&&r.memoizedState.cachePool!==null&&(a=r.memoizedState.cachePool.pool),a!==i&&(r.flags|=2048)),n!==e&&n&&(t.child.flags|=8192),Hc(t,t.updateQueue),Wc(t),null);case 4:return xe(),e===null&&Td(t.stateNode.containerInfo),Wc(t),null;case 10:return ra(t.type),Wc(t),null;case 19:if(me(_o),r=t.memoizedState,r===null)return Wc(t),null;if(i=(t.flags&128)!=0,a=r.rendering,a===null)if(i)Uc(r,!1);else{if(Jl!==0||e!==null&&e.flags&128)for(e=t.child;e!==null;){if(a=vo(e),a!==null){for(t.flags|=128,Uc(r,!1),e=a.updateQueue,t.updateQueue=e,Hc(t,e),t.subtreeFlags=0,e=n,n=t.child;n!==null;)yi(n,e),n=n.sibling;return he(_o,_o.current&1|2),j&&Ii(t,r.treeForkCount),t.child}e=e.sibling}r.tail!==null&&Ie()>au&&(t.flags|=128,i=!0,Uc(r,!1),t.lanes=4194304)}else{if(!i)if(e=vo(a),e!==null){if(t.flags|=128,i=!0,e=e.updateQueue,t.updateQueue=e,Hc(t,e),Uc(r,!0),r.tail===null&&r.tailMode===`hidden`&&!a.alternate&&!j)return Wc(t),null}else 2*Ie()-r.renderingStartTime>au&&n!==536870912&&(t.flags|=128,i=!0,Uc(r,!1),t.lanes=4194304);r.isBackwards?(a.sibling=t.child,t.child=a):(e=r.last,e===null?t.child=a:e.sibling=a,r.last=a)}return r.tail===null?(Wc(t),null):(e=r.tail,r.rendering=e,r.tail=e.sibling,r.renderingStartTime=Ie(),e.sibling=null,n=_o.current,he(_o,i?n&1|2:n&1),j&&Ii(t,r.treeForkCount),e);case 22:case 23:return go(t),lo(),r=t.memoizedState!==null,e===null?r&&(t.flags|=8192):e.memoizedState!==null!==r&&(t.flags|=8192),r?n&536870912&&!(t.flags&128)&&(Wc(t),t.subtreeFlags&6&&(t.flags|=8192)):Wc(t),n=t.updateQueue,n!==null&&Hc(t,n.retryQueue),n=null,e!==null&&e.memoizedState!==null&&e.memoizedState.cachePool!==null&&(n=e.memoizedState.cachePool.pool),r=null,t.memoizedState!==null&&t.memoizedState.cachePool!==null&&(r=t.memoizedState.cachePool.pool),r!==n&&(t.flags|=2048),e!==null&&me(Ta),null;case 24:return n=null,e!==null&&(n=e.memoizedState.cache),t.memoizedState.cache!==n&&(t.flags|=2048),ra(ma),Wc(t),null;case 25:return null;case 30:return null}throw Error(s(156,t.tag))}function Kc(e,t){switch(zi(t),t.tag){case 1:return e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 3:return ra(ma),xe(),e=t.flags,e&65536&&!(e&128)?(t.flags=e&-65537|128,t):null;case 26:case 27:case 5:return Ce(t),null;case 31:if(t.memoizedState!==null){if(go(t),t.alternate===null)throw Error(s(340));Xi()}return e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 13:if(go(t),e=t.memoizedState,e!==null&&e.dehydrated!==null){if(t.alternate===null)throw Error(s(340));Xi()}return e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 19:return me(_o),null;case 4:return xe(),null;case 10:return ra(t.type),null;case 22:case 23:return go(t),lo(),e!==null&&me(Ta),e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 24:return ra(ma),null;case 25:return null;default:return null}}function qc(e,t){switch(zi(t),t.tag){case 3:ra(ma),xe();break;case 26:case 27:case 5:Ce(t);break;case 4:xe();break;case 31:t.memoizedState!==null&&go(t);break;case 13:go(t);break;case 19:me(_o);break;case 10:ra(t.type);break;case 22:case 23:go(t),lo(),e!==null&&me(Ta);break;case 24:ra(ma)}}function Jc(e,t){try{var n=t.updateQueue,r=n===null?null:n.lastEffect;if(r!==null){var i=r.next;n=i;do{if((n.tag&e)===e){r=void 0;var a=n.create,o=n.inst;r=a(),o.destroy=r}n=n.next}while(n!==i)}}catch(e){Ju(t,t.return,e)}}function Yc(e,t,n){try{var r=t.updateQueue,i=r===null?null:r.lastEffect;if(i!==null){var a=i.next;r=a;do{if((r.tag&e)===e){var o=r.inst,s=o.destroy;if(s!==void 0){o.destroy=void 0,i=t;var c=n,l=s;try{l()}catch(e){Ju(i,c,e)}}}r=r.next}while(r!==a)}}catch(e){Ju(t,t.return,e)}}function Xc(e){var t=e.updateQueue;if(t!==null){var n=e.stateNode;try{io(t,n)}catch(t){Ju(e,e.return,t)}}}function Zc(e,t,n){n.props=$s(e.type,e.memoizedProps),n.state=e.memoizedState;try{n.componentWillUnmount()}catch(n){Ju(e,t,n)}}function Qc(e,t){try{var n=e.ref;if(n!==null){switch(e.tag){case 26:case 27:case 5:var r=e.stateNode;break;case 30:r=e.stateNode;break;default:r=e.stateNode}typeof n==`function`?e.refCleanup=n(r):n.current=r}}catch(n){Ju(e,t,n)}}function $c(e,t){var n=e.ref,r=e.refCleanup;if(n!==null)if(typeof r==`function`)try{r()}catch(n){Ju(e,t,n)}finally{e.refCleanup=null,e=e.alternate,e!=null&&(e.refCleanup=null)}else if(typeof n==`function`)try{n(null)}catch(n){Ju(e,t,n)}else n.current=null}function R(e){var t=e.type,n=e.memoizedProps,r=e.stateNode;try{a:switch(t){case`button`:case`input`:case`select`:case`textarea`:n.autoFocus&&r.focus();break a;case`img`:n.src?r.src=n.src:n.srcSet&&(r.srcset=n.srcSet)}}catch(t){Ju(e,e.return,t)}}function el(e,t,n){try{var r=e.stateNode;zd(r,e.type,n,t),r[vt]=t}catch(t){Ju(e,e.return,t)}}function tl(e){return e.tag===5||e.tag===3||e.tag===26||e.tag===27&&tf(e.type)||e.tag===4}function nl(e){a:for(;;){for(;e.sibling===null;){if(e.return===null||tl(e.return))return null;e=e.return}for(e.sibling.return=e.return,e=e.sibling;e.tag!==5&&e.tag!==6&&e.tag!==18;){if(e.tag===27&&tf(e.type)||e.flags&2||e.child===null||e.tag===4)continue a;e.child.return=e,e=e.child}if(!(e.flags&2))return e.stateNode}}function rl(e,t,n){var r=e.tag;if(r===5||r===6)e=e.stateNode,t?(n.nodeType===9?n.body:n.nodeName===`HTML`?n.ownerDocument.body:n).insertBefore(e,t):(t=n.nodeType===9?n.body:n.nodeName===`HTML`?n.ownerDocument.body:n,t.appendChild(e),n=n._reactRootContainer,n!=null||t.onclick!==null||(t.onclick=un));else if(r!==4&&(r===27&&tf(e.type)&&(n=e.stateNode,t=null),e=e.child,e!==null))for(rl(e,t,n),e=e.sibling;e!==null;)rl(e,t,n),e=e.sibling}function il(e,t,n){var r=e.tag;if(r===5||r===6)e=e.stateNode,t?n.insertBefore(e,t):n.appendChild(e);else if(r!==4&&(r===27&&tf(e.type)&&(n=e.stateNode),e=e.child,e!==null))for(il(e,t,n),e=e.sibling;e!==null;)il(e,t,n),e=e.sibling}function al(e){var t=e.stateNode,n=e.memoizedProps;try{for(var r=e.type,i=t.attributes;i.length;)t.removeAttributeNode(i[0]);Rd(t,r,n),t[_t]=e,t[vt]=n}catch(t){Ju(e,e.return,t)}}var ol=!1,sl=!1,cl=!1,ll=typeof WeakSet==`function`?WeakSet:Set,z=null;function ul(e,t){if(e=e.containerInfo,Hd=up,e=Ir(e),Lr(e)){if(`selectionStart`in e)var n={start:e.selectionStart,end:e.selectionEnd};else a:{n=(n=e.ownerDocument)&&n.defaultView||window;var r=n.getSelection&&n.getSelection();if(r&&r.rangeCount!==0){n=r.anchorNode;var i=r.anchorOffset,a=r.focusNode;r=r.focusOffset;try{n.nodeType,a.nodeType}catch{n=null;break a}var o=0,c=-1,l=-1,u=0,d=0,f=e,p=null;b:for(;;){for(var m;f!==n||i!==0&&f.nodeType!==3||(c=o+i),f!==a||r!==0&&f.nodeType!==3||(l=o+r),f.nodeType===3&&(o+=f.nodeValue.length),(m=f.firstChild)!==null;)p=f,f=m;for(;;){if(f===e)break b;if(p===n&&++u===i&&(c=o),p===a&&++d===r&&(l=o),(m=f.nextSibling)!==null)break;f=p,p=f.parentNode}f=m}n=c===-1||l===-1?null:{start:c,end:l}}else n=null}n||={start:0,end:0}}else n=null;for(Ud={focusedElem:e,selectionRange:n},up=!1,z=t;z!==null;)if(t=z,e=t.child,t.subtreeFlags&1028&&e!==null)e.return=t,z=e;else for(;z!==null;){switch(t=z,a=t.alternate,e=t.flags,t.tag){case 0:if(e&4&&(e=t.updateQueue,e=e===null?null:e.events,e!==null))for(n=0;n<e.length;n++)i=e[n],i.ref.impl=i.nextImpl;break;case 11:case 15:break;case 1:if(e&1024&&a!==null){e=void 0,n=t,i=a.memoizedProps,a=a.memoizedState,r=n.stateNode;try{var h=$s(n.type,i);e=r.getSnapshotBeforeUpdate(h,a),r.__reactInternalSnapshotBeforeUpdate=e}catch(e){Ju(n,n.return,e)}}break;case 3:if(e&1024){if(e=t.stateNode.containerInfo,n=e.nodeType,n===9)af(e);else if(n===1)switch(e.nodeName){case`HEAD`:case`HTML`:case`BODY`:af(e);break;default:e.textContent=``}}break;case 5:case 26:case 27:case 6:case 4:case 17:break;default:if(e&1024)throw Error(s(163))}if(e=t.sibling,e!==null){e.return=t.return,z=e;break}z=t.return}}function dl(e,t,n){var r=n.flags;switch(n.tag){case 0:case 11:case 15:B(e,n),r&4&&Jc(5,n);break;case 1:if(B(e,n),r&4)if(e=n.stateNode,t===null)try{e.componentDidMount()}catch(e){Ju(n,n.return,e)}else{var i=$s(n.type,t.memoizedProps);t=t.memoizedState;try{e.componentDidUpdate(i,t,e.__reactInternalSnapshotBeforeUpdate)}catch(e){Ju(n,n.return,e)}}r&64&&Xc(n),r&512&&Qc(n,n.return);break;case 3:if(B(e,n),r&64&&(e=n.updateQueue,e!==null)){if(t=null,n.child!==null)switch(n.child.tag){case 27:case 5:t=n.child.stateNode;break;case 1:t=n.child.stateNode}try{io(e,t)}catch(e){Ju(n,n.return,e)}}break;case 27:t===null&&r&4&&al(n);case 26:case 5:B(e,n),t===null&&r&4&&R(n),r&512&&Qc(n,n.return);break;case 12:B(e,n);break;case 31:B(e,n),r&4&&_l(e,n);break;case 13:B(e,n),r&4&&vl(e,n),r&64&&(e=n.memoizedState,e!==null&&(e=e.dehydrated,e!==null&&(n=Qu.bind(null,n),uf(e,n))));break;case 22:if(r=n.memoizedState!==null||ol,!r){t=t!==null&&t.memoizedState!==null||sl,i=ol;var a=sl;ol=r,(sl=t)&&!a?Dl(e,n,(n.subtreeFlags&8772)!=0):B(e,n),ol=i,sl=a}break;case 30:break;default:B(e,n)}}function fl(e){var t=e.alternate;t!==null&&(e.alternate=null,fl(t)),e.child=null,e.deletions=null,e.sibling=null,e.tag===5&&(t=e.stateNode,t!==null&&Tt(t)),e.stateNode=null,e.return=null,e.dependencies=null,e.memoizedProps=null,e.memoizedState=null,e.pendingProps=null,e.stateNode=null,e.updateQueue=null}var pl=null,ml=!1;function hl(e,t,n){for(n=n.child;n!==null;)gl(e,t,n),n=n.sibling}function gl(e,t,n){if(Ke&&typeof Ke.onCommitFiberUnmount==`function`)try{Ke.onCommitFiberUnmount(Ge,n)}catch{}switch(n.tag){case 26:sl||$c(n,t),hl(e,t,n),n.memoizedState?n.memoizedState.count--:n.stateNode&&(n=n.stateNode,n.parentNode.removeChild(n));break;case 27:sl||$c(n,t);var r=pl,i=ml;tf(n.type)&&(pl=n.stateNode,ml=!1),hl(e,t,n),gf(n.stateNode),pl=r,ml=i;break;case 5:sl||$c(n,t);case 6:if(r=pl,i=ml,pl=null,hl(e,t,n),pl=r,ml=i,pl!==null)if(ml)try{(pl.nodeType===9?pl.body:pl.nodeName===`HTML`?pl.ownerDocument.body:pl).removeChild(n.stateNode)}catch(e){Ju(n,t,e)}else try{pl.removeChild(n.stateNode)}catch(e){Ju(n,t,e)}break;case 18:pl!==null&&(ml?(e=pl,nf(e.nodeType===9?e.body:e.nodeName===`HTML`?e.ownerDocument.body:e,n.stateNode),$(e)):nf(pl,n.stateNode));break;case 4:r=pl,i=ml,pl=n.stateNode.containerInfo,ml=!0,hl(e,t,n),pl=r,ml=i;break;case 0:case 11:case 14:case 15:Yc(2,n,t),sl||Yc(4,n,t),hl(e,t,n);break;case 1:sl||($c(n,t),r=n.stateNode,typeof r.componentWillUnmount==`function`&&Zc(n,t,r)),hl(e,t,n);break;case 21:hl(e,t,n);break;case 22:sl=(r=sl)||n.memoizedState!==null,hl(e,t,n),sl=r;break;default:hl(e,t,n)}}function _l(e,t){if(t.memoizedState===null&&(e=t.alternate,e!==null&&(e=e.memoizedState,e!==null))){e=e.dehydrated;try{$(e)}catch(e){Ju(t,t.return,e)}}}function vl(e,t){if(t.memoizedState===null&&(e=t.alternate,e!==null&&(e=e.memoizedState,e!==null&&(e=e.dehydrated,e!==null))))try{$(e)}catch(e){Ju(t,t.return,e)}}function yl(e){switch(e.tag){case 31:case 13:case 19:var t=e.stateNode;return t===null&&(t=e.stateNode=new ll),t;case 22:return e=e.stateNode,t=e._retryCache,t===null&&(t=e._retryCache=new ll),t;default:throw Error(s(435,e.tag))}}function bl(e,t){var n=yl(e);t.forEach(function(t){if(!n.has(t)){n.add(t);var r=$u.bind(null,e,t);t.then(r,r)}})}function xl(e,t){var n=t.deletions;if(n!==null)for(var r=0;r<n.length;r++){var i=n[r],a=e,o=t,c=o;a:for(;c!==null;){switch(c.tag){case 27:if(tf(c.type)){pl=c.stateNode,ml=!1;break a}break;case 5:pl=c.stateNode,ml=!1;break a;case 3:case 4:pl=c.stateNode.containerInfo,ml=!0;break a}c=c.return}if(pl===null)throw Error(s(160));gl(a,o,i),pl=null,ml=!1,a=i.alternate,a!==null&&(a.return=null),i.return=null}if(t.subtreeFlags&13886)for(t=t.child;t!==null;)Cl(t,e),t=t.sibling}var Sl=null;function Cl(e,t){var n=e.alternate,r=e.flags;switch(e.tag){case 0:case 11:case 14:case 15:xl(t,e),wl(e),r&4&&(Yc(3,e,e.return),Jc(3,e),Yc(5,e,e.return));break;case 1:xl(t,e),wl(e),r&512&&(sl||n===null||$c(n,n.return)),r&64&&ol&&(e=e.updateQueue,e!==null&&(r=e.callbacks,r!==null&&(n=e.shared.hiddenCallbacks,e.shared.hiddenCallbacks=n===null?r:n.concat(r))));break;case 26:var i=Sl;if(xl(t,e),wl(e),r&512&&(sl||n===null||$c(n,n.return)),r&4){var a=n===null?null:n.memoizedState;if(r=e.memoizedState,n===null)if(r===null)if(e.stateNode===null){a:{r=e.type,n=e.memoizedProps,i=i.ownerDocument||i;b:switch(r){case`title`:a=i.getElementsByTagName(`title`)[0],(!a||a[wt]||a[_t]||a.namespaceURI===`http://www.w3.org/2000/svg`||a.hasAttribute(`itemprop`))&&(a=i.createElement(r),i.head.insertBefore(a,i.querySelector(`head > title`))),Rd(a,r,n),a[_t]=e,At(a),r=a;break a;case`link`:var o=Wf(`link`,`href`,i).get(r+(n.href||``));if(o){for(var c=0;c<o.length;c++)if(a=o[c],a.getAttribute(`href`)===(n.href==null||n.href===``?null:n.href)&&a.getAttribute(`rel`)===(n.rel==null?null:n.rel)&&a.getAttribute(`title`)===(n.title==null?null:n.title)&&a.getAttribute(`crossorigin`)===(n.crossOrigin==null?null:n.crossOrigin)){o.splice(c,1);break b}}a=i.createElement(r),Rd(a,r,n),i.head.appendChild(a);break;case`meta`:if(o=Wf(`meta`,`content`,i).get(r+(n.content||``))){for(c=0;c<o.length;c++)if(a=o[c],a.getAttribute(`content`)===(n.content==null?null:``+n.content)&&a.getAttribute(`name`)===(n.name==null?null:n.name)&&a.getAttribute(`property`)===(n.property==null?null:n.property)&&a.getAttribute(`http-equiv`)===(n.httpEquiv==null?null:n.httpEquiv)&&a.getAttribute(`charset`)===(n.charSet==null?null:n.charSet)){o.splice(c,1);break b}}a=i.createElement(r),Rd(a,r,n),i.head.appendChild(a);break;default:throw Error(s(468,r))}a[_t]=e,At(a),r=a}e.stateNode=r}else Gf(i,e.type,e.stateNode);else e.stateNode=zf(i,r,e.memoizedProps);else a===r?r===null&&e.stateNode!==null&&el(e,e.memoizedProps,n.memoizedProps):(a===null?n.stateNode!==null&&(n=n.stateNode,n.parentNode.removeChild(n)):a.count--,r===null?Gf(i,e.type,e.stateNode):zf(i,r,e.memoizedProps))}break;case 27:xl(t,e),wl(e),r&512&&(sl||n===null||$c(n,n.return)),n!==null&&r&4&&el(e,e.memoizedProps,n.memoizedProps);break;case 5:if(xl(t,e),wl(e),r&512&&(sl||n===null||$c(n,n.return)),e.flags&32){i=e.stateNode;try{O(i,``)}catch(t){Ju(e,e.return,t)}}r&4&&e.stateNode!=null&&(i=e.memoizedProps,el(e,i,n===null?i:n.memoizedProps)),r&1024&&(cl=!0);break;case 6:if(xl(t,e),wl(e),r&4){if(e.stateNode===null)throw Error(s(162));r=e.memoizedProps,n=e.stateNode;try{n.nodeValue=r}catch(t){Ju(e,e.return,t)}}break;case 3:if(Uf=null,i=Sl,Sl=yf(t.containerInfo),xl(t,e),Sl=i,wl(e),r&4&&n!==null&&n.memoizedState.isDehydrated)try{$(t.containerInfo)}catch(t){Ju(e,e.return,t)}cl&&(cl=!1,Tl(e));break;case 4:r=Sl,Sl=yf(e.stateNode.containerInfo),xl(t,e),wl(e),Sl=r;break;case 12:xl(t,e),wl(e);break;case 31:xl(t,e),wl(e),r&4&&(r=e.updateQueue,r!==null&&(e.updateQueue=null,bl(e,r)));break;case 13:xl(t,e),wl(e),e.child.flags&8192&&e.memoizedState!==null!=(n!==null&&n.memoizedState!==null)&&(ru=Ie()),r&4&&(r=e.updateQueue,r!==null&&(e.updateQueue=null,bl(e,r)));break;case 22:i=e.memoizedState!==null;var l=n!==null&&n.memoizedState!==null,u=ol,d=sl;if(ol=u||i,sl=d||l,xl(t,e),sl=d,ol=u,wl(e),r&8192)a:for(t=e.stateNode,t._visibility=i?t._visibility&-2:t._visibility|1,i&&(n===null||l||ol||sl||El(e)),n=null,t=e;;){if(t.tag===5||t.tag===26){if(n===null){l=n=t;try{if(a=l.stateNode,i)o=a.style,typeof o.setProperty==`function`?o.setProperty(`display`,`none`,`important`):o.display=`none`;else{c=l.stateNode;var f=l.memoizedProps.style,p=f!=null&&f.hasOwnProperty(`display`)?f.display:null;c.style.display=p==null||typeof p==`boolean`?``:(``+p).trim()}}catch(e){Ju(l,l.return,e)}}}else if(t.tag===6){if(n===null){l=t;try{l.stateNode.nodeValue=i?``:l.memoizedProps}catch(e){Ju(l,l.return,e)}}}else if(t.tag===18){if(n===null){l=t;try{var m=l.stateNode;i?rf(m,!0):rf(l.stateNode,!1)}catch(e){Ju(l,l.return,e)}}}else if((t.tag!==22&&t.tag!==23||t.memoizedState===null||t===e)&&t.child!==null){t.child.return=t,t=t.child;continue}if(t===e)break a;for(;t.sibling===null;){if(t.return===null||t.return===e)break a;n===t&&(n=null),t=t.return}n===t&&(n=null),t.sibling.return=t.return,t=t.sibling}r&4&&(r=e.updateQueue,r!==null&&(n=r.retryQueue,n!==null&&(r.retryQueue=null,bl(e,n))));break;case 19:xl(t,e),wl(e),r&4&&(r=e.updateQueue,r!==null&&(e.updateQueue=null,bl(e,r)));break;case 30:break;case 21:break;default:xl(t,e),wl(e)}}function wl(e){var t=e.flags;if(t&2){try{for(var n,r=e.return;r!==null;){if(tl(r)){n=r;break}r=r.return}if(n==null)throw Error(s(160));switch(n.tag){case 27:var i=n.stateNode;il(e,nl(e),i);break;case 5:var a=n.stateNode;n.flags&32&&(O(a,``),n.flags&=-33),il(e,nl(e),a);break;case 3:case 4:var o=n.stateNode.containerInfo;rl(e,nl(e),o);break;default:throw Error(s(161))}}catch(t){Ju(e,e.return,t)}e.flags&=-3}t&4096&&(e.flags&=-4097)}function Tl(e){if(e.subtreeFlags&1024)for(e=e.child;e!==null;){var t=e;Tl(t),t.tag===5&&t.flags&1024&&t.stateNode.reset(),e=e.sibling}}function B(e,t){if(t.subtreeFlags&8772)for(t=t.child;t!==null;)dl(e,t.alternate,t),t=t.sibling}function El(e){for(e=e.child;e!==null;){var t=e;switch(t.tag){case 0:case 11:case 14:case 15:Yc(4,t,t.return),El(t);break;case 1:$c(t,t.return);var n=t.stateNode;typeof n.componentWillUnmount==`function`&&Zc(t,t.return,n),El(t);break;case 27:gf(t.stateNode);case 26:case 5:$c(t,t.return),El(t);break;case 22:t.memoizedState===null&&El(t);break;case 30:El(t);break;default:El(t)}e=e.sibling}}function Dl(e,t,n){for(n&&=(t.subtreeFlags&8772)!=0,t=t.child;t!==null;){var r=t.alternate,i=e,a=t,o=a.flags;switch(a.tag){case 0:case 11:case 15:Dl(i,a,n),Jc(4,a);break;case 1:if(Dl(i,a,n),r=a,i=r.stateNode,typeof i.componentDidMount==`function`)try{i.componentDidMount()}catch(e){Ju(r,r.return,e)}if(r=a,i=r.updateQueue,i!==null){var s=r.stateNode;try{var c=i.shared.hiddenCallbacks;if(c!==null)for(i.shared.hiddenCallbacks=null,i=0;i<c.length;i++)ro(c[i],s)}catch(e){Ju(r,r.return,e)}}n&&o&64&&Xc(a),Qc(a,a.return);break;case 27:al(a);case 26:case 5:Dl(i,a,n),n&&r===null&&o&4&&R(a),Qc(a,a.return);break;case 12:Dl(i,a,n);break;case 31:Dl(i,a,n),n&&o&4&&_l(i,a);break;case 13:Dl(i,a,n),n&&o&4&&vl(i,a);break;case 22:a.memoizedState===null&&Dl(i,a,n),Qc(a,a.return);break;case 30:break;default:Dl(i,a,n)}t=t.sibling}}function Ol(e,t){var n=null;e!==null&&e.memoizedState!==null&&e.memoizedState.cachePool!==null&&(n=e.memoizedState.cachePool.pool),e=null,t.memoizedState!==null&&t.memoizedState.cachePool!==null&&(e=t.memoizedState.cachePool.pool),e!==n&&(e!=null&&e.refCount++,n!=null&&ga(n))}function kl(e,t){e=null,t.alternate!==null&&(e=t.alternate.memoizedState.cache),t=t.memoizedState.cache,t!==e&&(t.refCount++,e!=null&&ga(e))}function Al(e,t,n,r){if(t.subtreeFlags&10256)for(t=t.child;t!==null;)jl(e,t,n,r),t=t.sibling}function jl(e,t,n,r){var i=t.flags;switch(t.tag){case 0:case 11:case 15:Al(e,t,n,r),i&2048&&Jc(9,t);break;case 1:Al(e,t,n,r);break;case 3:Al(e,t,n,r),i&2048&&(e=null,t.alternate!==null&&(e=t.alternate.memoizedState.cache),t=t.memoizedState.cache,t!==e&&(t.refCount++,e!=null&&ga(e)));break;case 12:if(i&2048){Al(e,t,n,r),e=t.stateNode;try{var a=t.memoizedProps,o=a.id,s=a.onPostCommit;typeof s==`function`&&s(o,t.alternate===null?`mount`:`update`,e.passiveEffectDuration,-0)}catch(e){Ju(t,t.return,e)}}else Al(e,t,n,r);break;case 31:Al(e,t,n,r);break;case 13:Al(e,t,n,r);break;case 23:break;case 22:a=t.stateNode,o=t.alternate,t.memoizedState===null?a._visibility&2?Al(e,t,n,r):(a._visibility|=2,Ml(e,t,n,r,(t.subtreeFlags&10256)!=0||!1)):a._visibility&2?Al(e,t,n,r):Nl(e,t),i&2048&&Ol(o,t);break;case 24:Al(e,t,n,r),i&2048&&kl(t.alternate,t);break;default:Al(e,t,n,r)}}function Ml(e,t,n,r,i){for(i&&=(t.subtreeFlags&10256)!=0||!1,t=t.child;t!==null;){var a=e,o=t,s=n,c=r,l=o.flags;switch(o.tag){case 0:case 11:case 15:Ml(a,o,s,c,i),Jc(8,o);break;case 23:break;case 22:var u=o.stateNode;o.memoizedState===null?(u._visibility|=2,Ml(a,o,s,c,i)):u._visibility&2?Ml(a,o,s,c,i):Nl(a,o),i&&l&2048&&Ol(o.alternate,o);break;case 24:Ml(a,o,s,c,i),i&&l&2048&&kl(o.alternate,o);break;default:Ml(a,o,s,c,i)}t=t.sibling}}function Nl(e,t){if(t.subtreeFlags&10256)for(t=t.child;t!==null;){var n=e,r=t,i=r.flags;switch(r.tag){case 22:Nl(n,r),i&2048&&Ol(r.alternate,r);break;case 24:Nl(n,r),i&2048&&kl(r.alternate,r);break;default:Nl(n,r)}t=t.sibling}}var Pl=8192;function Fl(e,t,n){if(e.subtreeFlags&Pl)for(e=e.child;e!==null;)Il(e,t,n),e=e.sibling}function Il(e,t,n){switch(e.tag){case 26:Fl(e,t,n),e.flags&Pl&&e.memoizedState!==null&&Jf(n,Sl,e.memoizedState,e.memoizedProps);break;case 5:Fl(e,t,n);break;case 3:case 4:var r=Sl;Sl=yf(e.stateNode.containerInfo),Fl(e,t,n),Sl=r;break;case 22:e.memoizedState===null&&(r=e.alternate,r!==null&&r.memoizedState!==null?(r=Pl,Pl=16777216,Fl(e,t,n),Pl=r):Fl(e,t,n));break;default:Fl(e,t,n)}}function Ll(e){var t=e.alternate;if(t!==null&&(e=t.child,e!==null)){t.child=null;do t=e.sibling,e.sibling=null,e=t;while(e!==null)}}function Rl(e){var t=e.deletions;if(e.flags&16){if(t!==null)for(var n=0;n<t.length;n++){var r=t[n];z=r,Bl(r,e)}Ll(e)}if(e.subtreeFlags&10256)for(e=e.child;e!==null;)V(e),e=e.sibling}function V(e){switch(e.tag){case 0:case 11:case 15:Rl(e),e.flags&2048&&Yc(9,e,e.return);break;case 3:Rl(e);break;case 12:Rl(e);break;case 22:var t=e.stateNode;e.memoizedState!==null&&t._visibility&2&&(e.return===null||e.return.tag!==13)?(t._visibility&=-3,zl(e)):Rl(e);break;default:Rl(e)}}function zl(e){var t=e.deletions;if(e.flags&16){if(t!==null)for(var n=0;n<t.length;n++){var r=t[n];z=r,Bl(r,e)}Ll(e)}for(e=e.child;e!==null;){switch(t=e,t.tag){case 0:case 11:case 15:Yc(8,t,t.return),zl(t);break;case 22:n=t.stateNode,n._visibility&2&&(n._visibility&=-3,zl(t));break;default:zl(t)}e=e.sibling}}function Bl(e,t){for(;z!==null;){var n=z;switch(n.tag){case 0:case 11:case 15:Yc(8,n,t);break;case 23:case 22:if(n.memoizedState!==null&&n.memoizedState.cachePool!==null){var r=n.memoizedState.cachePool.pool;r!=null&&r.refCount++}break;case 24:ga(n.memoizedState.cache)}if(r=n.child,r!==null)r.return=n,z=r;else a:for(n=e;z!==null;){r=z;var i=r.sibling,a=r.return;if(fl(r),r===n){z=null;break a}if(i!==null){i.return=a,z=i;break a}z=a}}}var Vl={getCacheForType:function(e){var t=ca(ma),n=t.data.get(e);return n===void 0&&(n=e(),t.data.set(e,n)),n},cacheSignal:function(){return ca(ma).controller.signal}},Hl=typeof WeakMap==`function`?WeakMap:Map,H=0,U=null,W=null,G=0,Ul=0,Wl=null,Gl=!1,K=!1,Kl=!1,ql=0,Jl=0,Yl=0,Xl=0,Zl=0,Ql=0,$l=0,eu=null,tu=null,nu=!1,ru=0,iu=0,au=1/0,ou=null,q=null,su=0,cu=null,lu=null,uu=0,du=0,fu=null,pu=null,mu=0,hu=null;function gu(){return H&2&&G!==0?G&-G:E.T===null?mt():hd()}function _u(){if(Ql===0)if(!(G&536870912)||j){var e=$e;$e<<=1,!($e&3932160)&&($e=262144),Ql=e}else Ql=536870912;return e=uo.current,e!==null&&(e.flags|=32),Ql}function vu(e,t,n){(e===U&&(Ul===2||Ul===9)||e.cancelPendingCommit!==null)&&(Tu(e,0),Su(e,G,Ql,!1)),st(e,n),(!(H&2)||e!==U)&&(e===U&&(!(H&2)&&(Xl|=n),Jl===4&&Su(e,G,Ql,!1)),sd(e))}function yu(e,t,n){if(H&6)throw Error(s(327));var r=!n&&(t&127)==0&&(t&e.expiredLanes)===0||rt(e,t),i=r?Nu(e,t):ju(e,t,!0),a=r;do{if(i===0){K&&!r&&Su(e,t,0,!1);break}else{if(n=e.current.alternate,a&&!xu(n)){i=ju(e,t,!1),a=!1;continue}if(i===2){if(a=t,e.errorRecoveryDisabledLanes&a)var o=0;else o=e.pendingLanes&-536870913,o=o===0?o&536870912?536870912:0:o;if(o!==0){t=o;a:{var c=e;i=eu;var l=c.current.memoizedState.isDehydrated;if(l&&(Tu(c,o).flags|=256),o=ju(c,o,!1),o!==2){if(Kl&&!l){c.errorRecoveryDisabledLanes|=a,Xl|=a,i=4;break a}a=tu,tu=i,a!==null&&(tu===null?tu=a:tu.push.apply(tu,a))}i=o}if(a=!1,i!==2)continue}}if(i===1){Tu(e,0),Su(e,t,0,!0);break}a:{switch(r=e,a=i,a){case 0:case 1:throw Error(s(345));case 4:if((t&4194048)!==t)break;case 6:Su(r,t,Ql,!Gl);break a;case 2:tu=null;break;case 3:case 5:break;default:throw Error(s(329))}if((t&62914560)===t&&(i=ru+300-Ie(),10<i)){if(Su(r,t,Ql,!Gl),nt(r,0,!0)!==0)break a;uu=t,r.timeoutHandle=Xd(bu.bind(null,r,n,tu,ou,nu,t,Ql,Xl,$l,Gl,a,`Throttled`,-0,0),i);break a}bu(r,n,tu,ou,nu,t,Ql,Xl,$l,Gl,a,null,-0,0)}}break}while(1);sd(e)}function bu(e,t,n,r,i,a,o,s,c,l,u,d,f,p){if(e.timeoutHandle=-1,d=t.subtreeFlags,d&8192||(d&16785408)==16785408){d={stylesheets:null,count:0,imgCount:0,imgBytes:0,suspenseyImages:[],waitingForImages:!0,waitingForViewTransition:!1,unsuspend:un},Il(t,a,d);var m=(a&62914560)===a?ru-Ie():(a&4194048)===a?iu-Ie():0;if(m=Xf(d,m),m!==null){uu=a,e.cancelPendingCommit=m(Bu.bind(null,e,t,a,n,r,i,o,s,c,u,d,null,f,p)),Su(e,a,o,!l);return}}Bu(e,t,a,n,r,i,o,s,c)}function xu(e){for(var t=e;;){var n=t.tag;if((n===0||n===11||n===15)&&t.flags&16384&&(n=t.updateQueue,n!==null&&(n=n.stores,n!==null)))for(var r=0;r<n.length;r++){var i=n[r],a=i.getSnapshot;i=i.value;try{if(!jr(a(),i))return!1}catch{return!1}}if(n=t.child,t.subtreeFlags&16384&&n!==null)n.return=t,t=n;else{if(t===e)break;for(;t.sibling===null;){if(t.return===null||t.return===e)return!0;t=t.return}t.sibling.return=t.return,t=t.sibling}}return!0}function Su(e,t,n,r){t&=~Zl,t&=~Xl,e.suspendedLanes|=t,e.pingedLanes&=~t,r&&(e.warmLanes|=t),r=e.expirationTimes;for(var i=t;0<i;){var a=31-Je(i),o=1<<a;r[a]=-1,i&=~o}n!==0&&lt(e,n,t)}function Cu(){return H&6?!0:(cd(0,!1),!1)}function wu(){if(W!==null){if(Ul===0)var e=W.return;else e=W,ta=ea=null,Lo(e),za=null,Ba=0,e=W;for(;e!==null;)qc(e.alternate,e),e=e.return;W=null}}function Tu(e,t){var n=e.timeoutHandle;n!==-1&&(e.timeoutHandle=-1,Zd(n)),n=e.cancelPendingCommit,n!==null&&(e.cancelPendingCommit=null,n()),uu=0,wu(),U=e,W=n=vi(e.current,null),G=t,Ul=0,Wl=null,Gl=!1,K=rt(e,t),Kl=!1,$l=Ql=Zl=Xl=Yl=Jl=0,tu=eu=null,nu=!1,t&8&&(t|=t&32);var r=e.entangledLanes;if(r!==0)for(e=e.entanglements,r&=t;0<r;){var i=31-Je(r),a=1<<i;t|=e[i],r&=~a}return ql=t,ci(),n}function Eu(e,t){P=null,E.H=Gs,t===ka||t===ja?(t=La(),Ul=3):t===Aa?(t=La(),Ul=4):Ul=t===lc?8:typeof t==`object`&&t&&typeof t.then==`function`?6:1,Wl=t,W===null&&(Jl=1,rc(e,Ei(t,e.current)))}function Du(){var e=uo.current;return e===null?!0:(G&4194048)===G?fo===null:(G&62914560)===G||G&536870912?e===fo:!1}function Ou(){var e=E.H;return E.H=Gs,e===null?Gs:e}function ku(){var e=E.A;return E.A=Vl,e}function Au(){Jl=4,Gl||(G&4194048)!==G&&uo.current!==null||(K=!0),!(Yl&134217727)&&!(Xl&134217727)||U===null||Su(U,G,Ql,!1)}function ju(e,t,n){var r=H;H|=2;var i=Ou(),a=ku();(U!==e||G!==t)&&(ou=null,Tu(e,t)),t=!1;var o=Jl;a:do try{if(Ul!==0&&W!==null){var s=W,c=Wl;switch(Ul){case 8:wu(),o=6;break a;case 3:case 2:case 9:case 6:uo.current===null&&(t=!0);var l=Ul;if(Ul=0,Wl=null,Lu(e,s,c,l),n&&K){o=0;break a}break;default:l=Ul,Ul=0,Wl=null,Lu(e,s,c,l)}}Mu(),o=Jl;break}catch(t){Eu(e,t)}while(1);return t&&e.shellSuspendCounter++,ta=ea=null,H=r,E.H=i,E.A=a,W===null&&(U=null,G=0,ci()),o}function Mu(){for(;W!==null;)Fu(W)}function Nu(e,t){var n=H;H|=2;var r=Ou(),i=ku();U!==e||G!==t?(ou=null,au=Ie()+500,Tu(e,t)):K=rt(e,t);a:do try{if(Ul!==0&&W!==null){t=W;var a=Wl;b:switch(Ul){case 1:Ul=0,Wl=null,Lu(e,t,a,1);break;case 2:case 9:if(Na(a)){Ul=0,Wl=null,Iu(t);break}t=function(){Ul!==2&&Ul!==9||U!==e||(Ul=7),sd(e)},a.then(t,t);break a;case 3:Ul=7;break a;case 4:Ul=5;break a;case 7:Na(a)?(Ul=0,Wl=null,Iu(t)):(Ul=0,Wl=null,Lu(e,t,a,7));break;case 5:var o=null;switch(W.tag){case 26:o=W.memoizedState;case 5:case 27:var c=W;if(o?qf(o):c.stateNode.complete){Ul=0,Wl=null;var l=c.sibling;if(l!==null)W=l;else{var u=c.return;u===null?W=null:(W=u,Ru(u))}break b}}Ul=0,Wl=null,Lu(e,t,a,5);break;case 6:Ul=0,Wl=null,Lu(e,t,a,6);break;case 8:wu(),Jl=6;break a;default:throw Error(s(462))}}Pu();break}catch(t){Eu(e,t)}while(1);return ta=ea=null,E.H=r,E.A=i,H=n,W===null?(U=null,G=0,ci(),Jl):0}function Pu(){for(;W!==null&&!Pe();)Fu(W)}function Fu(e){var t=L(e.alternate,e,ql);e.memoizedProps=e.pendingProps,t===null?Ru(e):W=t}function Iu(e){var t=e,n=t.alternate;switch(t.tag){case 15:case 0:t=Cc(n,t,t.pendingProps,t.type,void 0,G);break;case 11:t=Cc(n,t,t.pendingProps,t.type.render,t.ref,G);break;case 5:Lo(t);default:qc(n,t),t=W=yi(t,ql),t=L(n,t,ql)}e.memoizedProps=e.pendingProps,t===null?Ru(e):W=t}function Lu(e,t,n,r){ta=ea=null,Lo(t),za=null,Ba=0;var i=t.return;try{if(cc(e,i,t,n,G)){Jl=1,rc(e,Ei(n,e.current)),W=null;return}}catch(t){if(i!==null)throw W=i,t;Jl=1,rc(e,Ei(n,e.current)),W=null;return}t.flags&32768?(j||r===1?e=!0:K||G&536870912?e=!1:(Gl=e=!0,(r===2||r===9||r===3||r===6)&&(r=uo.current,r!==null&&r.tag===13&&(r.flags|=16384))),zu(t,e)):Ru(t)}function Ru(e){var t=e;do{if(t.flags&32768){zu(t,Gl);return}e=t.return;var n=Gc(t.alternate,t,ql);if(n!==null){W=n;return}if(t=t.sibling,t!==null){W=t;return}W=t=e}while(t!==null);Jl===0&&(Jl=5)}function zu(e,t){do{var n=Kc(e.alternate,e);if(n!==null){n.flags&=32767,W=n;return}if(n=e.return,n!==null&&(n.flags|=32768,n.subtreeFlags=0,n.deletions=null),!t&&(e=e.sibling,e!==null)){W=e;return}W=e=n}while(e!==null);Jl=6,W=null}function Bu(e,t,n,r,i,a,o,c,l){e.cancelPendingCommit=null;do Gu();while(su!==0);if(H&6)throw Error(s(327));if(t!==null){if(t===e.current)throw Error(s(177));if(a=t.lanes|t.childLanes,a|=si,ct(e,n,a,o,c,l),e===U&&(W=U=null,G=0),lu=t,cu=e,uu=n,du=a,fu=i,pu=r,t.subtreeFlags&10256||t.flags&10256?(e.callbackNode=null,e.callbackPriority=0,ed(Be,function(){return Ku(),null})):(e.callbackNode=null,e.callbackPriority=0),r=(t.flags&13878)!=0,t.subtreeFlags&13878||r){r=E.T,E.T=null,i=D.p,D.p=2,o=H,H|=4;try{ul(e,t,n)}finally{H=o,D.p=i,E.T=r}}su=1,Vu(),Hu(),Uu()}}function Vu(){if(su===1){su=0;var e=cu,t=lu,n=(t.flags&13878)!=0;if(t.subtreeFlags&13878||n){n=E.T,E.T=null;var r=D.p;D.p=2;var i=H;H|=4;try{Cl(t,e);var a=Ud,o=Ir(e.containerInfo),s=a.focusedElem,c=a.selectionRange;if(o!==s&&s&&s.ownerDocument&&Fr(s.ownerDocument.documentElement,s)){if(c!==null&&Lr(s)){var l=c.start,u=c.end;if(u===void 0&&(u=l),`selectionStart`in s)s.selectionStart=l,s.selectionEnd=Math.min(u,s.value.length);else{var d=s.ownerDocument||document,f=d&&d.defaultView||window;if(f.getSelection){var p=f.getSelection(),m=s.textContent.length,h=Math.min(c.start,m),g=c.end===void 0?h:Math.min(c.end,m);!p.extend&&h>g&&(o=g,g=h,h=o);var _=Pr(s,h),v=Pr(s,g);if(_&&v&&(p.rangeCount!==1||p.anchorNode!==_.node||p.anchorOffset!==_.offset||p.focusNode!==v.node||p.focusOffset!==v.offset)){var y=d.createRange();y.setStart(_.node,_.offset),p.removeAllRanges(),h>g?(p.addRange(y),p.extend(v.node,v.offset)):(y.setEnd(v.node,v.offset),p.addRange(y))}}}}for(d=[],p=s;p=p.parentNode;)p.nodeType===1&&d.push({element:p,left:p.scrollLeft,top:p.scrollTop});for(typeof s.focus==`function`&&s.focus(),s=0;s<d.length;s++){var b=d[s];b.element.scrollLeft=b.left,b.element.scrollTop=b.top}}up=!!Hd,Ud=Hd=null}finally{H=i,D.p=r,E.T=n}}e.current=t,su=2}}function Hu(){if(su===2){su=0;var e=cu,t=lu,n=(t.flags&8772)!=0;if(t.subtreeFlags&8772||n){n=E.T,E.T=null;var r=D.p;D.p=2;var i=H;H|=4;try{dl(e,t.alternate,t)}finally{H=i,D.p=r,E.T=n}}su=3}}function Uu(){if(su===4||su===3){su=0,Fe();var e=cu,t=lu,n=uu,r=pu;t.subtreeFlags&10256||t.flags&10256?su=5:(su=0,lu=cu=null,Wu(e,e.pendingLanes));var i=e.pendingLanes;if(i===0&&(q=null),pt(n),t=t.stateNode,Ke&&typeof Ke.onCommitFiberRoot==`function`)try{Ke.onCommitFiberRoot(Ge,t,void 0,(t.current.flags&128)==128)}catch{}if(r!==null){t=E.T,i=D.p,D.p=2,E.T=null;try{for(var a=e.onRecoverableError,o=0;o<r.length;o++){var s=r[o];a(s.value,{componentStack:s.stack})}}finally{E.T=t,D.p=i}}uu&3&&Gu(),sd(e),i=e.pendingLanes,n&261930&&i&42?e===hu?mu++:(mu=0,hu=e):mu=0,cd(0,!1)}}function Wu(e,t){(e.pooledCacheLanes&=t)===0&&(t=e.pooledCache,t!=null&&(e.pooledCache=null,ga(t)))}function Gu(){return Vu(),Hu(),Uu(),Ku()}function Ku(){if(su!==5)return!1;var e=cu,t=du;du=0;var n=pt(uu),r=E.T,i=D.p;try{D.p=32>n?32:n,E.T=null,n=fu,fu=null;var a=cu,o=uu;if(su=0,lu=cu=null,uu=0,H&6)throw Error(s(331));var c=H;if(H|=4,V(a.current),jl(a,a.current,o,n),H=c,cd(0,!1),Ke&&typeof Ke.onPostCommitFiberRoot==`function`)try{Ke.onPostCommitFiberRoot(Ge,a)}catch{}return!0}finally{D.p=i,E.T=r,Wu(e,t)}}function qu(e,t,n){t=Ei(n,t),t=ac(e.stateNode,t,2),e=Za(e,t,2),e!==null&&(st(e,2),sd(e))}function Ju(e,t,n){if(e.tag===3)qu(e,e,n);else for(;t!==null;){if(t.tag===3){qu(t,e,n);break}else if(t.tag===1){var r=t.stateNode;if(typeof t.type.getDerivedStateFromError==`function`||typeof r.componentDidCatch==`function`&&(q===null||!q.has(r))){e=Ei(n,e),n=oc(2),r=Za(t,n,2),r!==null&&(sc(n,r,t,e),st(r,2),sd(r));break}}t=t.return}}function Yu(e,t,n){var r=e.pingCache;if(r===null){r=e.pingCache=new Hl;var i=new Set;r.set(t,i)}else i=r.get(t),i===void 0&&(i=new Set,r.set(t,i));i.has(n)||(Kl=!0,i.add(n),e=Xu.bind(null,e,t,n),t.then(e,e))}function Xu(e,t,n){var r=e.pingCache;r!==null&&r.delete(t),e.pingedLanes|=e.suspendedLanes&n,e.warmLanes&=~n,U===e&&(G&n)===n&&(Jl===4||Jl===3&&(G&62914560)===G&&300>Ie()-ru?!(H&2)&&Tu(e,0):Zl|=n,$l===G&&($l=0)),sd(e)}function Zu(e,t){t===0&&(t=at()),e=di(e,t),e!==null&&(st(e,t),sd(e))}function Qu(e){var t=e.memoizedState,n=0;t!==null&&(n=t.retryLane),Zu(e,n)}function $u(e,t){var n=0;switch(e.tag){case 31:case 13:var r=e.stateNode,i=e.memoizedState;i!==null&&(n=i.retryLane);break;case 19:r=e.stateNode;break;case 22:r=e.stateNode._retryCache;break;default:throw Error(s(314))}r!==null&&r.delete(t),Zu(e,n)}function ed(e,t){return Me(e,t)}var td=null,nd=null,rd=!1,id=!1,ad=!1,od=0;function sd(e){e!==nd&&e.next===null&&(nd===null?td=nd=e:nd=nd.next=e),id=!0,rd||(rd=!0,md())}function cd(e,t){if(!ad&&id){ad=!0;do for(var n=!1,r=td;r!==null;){if(!t)if(e!==0){var i=r.pendingLanes;if(i===0)var a=0;else{var o=r.suspendedLanes,s=r.pingedLanes;a=(1<<31-Je(42|e)+1)-1,a&=i&~(o&~s),a=a&201326741?a&201326741|1:a?a|2:0}a!==0&&(n=!0,pd(r,a))}else a=G,a=nt(r,r===U?a:0,r.cancelPendingCommit!==null||r.timeoutHandle!==-1),!(a&3)||rt(r,a)||(n=!0,pd(r,a));r=r.next}while(n);ad=!1}}function ld(){ud()}function ud(){id=rd=!1;var e=0;od!==0&&Yd()&&(e=od);for(var t=Ie(),n=null,r=td;r!==null;){var i=r.next,a=dd(r,t);a===0?(r.next=null,n===null?td=i:n.next=i,i===null&&(nd=n)):(n=r,(e!==0||a&3)&&(id=!0)),r=i}su!==0&&su!==5||cd(e,!1),od!==0&&(od=0)}function dd(e,t){for(var n=e.suspendedLanes,r=e.pingedLanes,i=e.expirationTimes,a=e.pendingLanes&-62914561;0<a;){var o=31-Je(a),s=1<<o,c=i[o];c===-1?((s&n)===0||(s&r)!==0)&&(i[o]=it(s,t)):c<=t&&(e.expiredLanes|=s),a&=~s}if(t=U,n=G,n=nt(e,e===t?n:0,e.cancelPendingCommit!==null||e.timeoutHandle!==-1),r=e.callbackNode,n===0||e===t&&(Ul===2||Ul===9)||e.cancelPendingCommit!==null)return r!==null&&r!==null&&Ne(r),e.callbackNode=null,e.callbackPriority=0;if(!(n&3)||rt(e,n)){if(t=n&-n,t===e.callbackPriority)return t;switch(r!==null&&Ne(r),pt(n)){case 2:case 8:n=ze;break;case 32:n=Be;break;case 268435456:n=He;break;default:n=Be}return r=fd.bind(null,e),n=Me(n,r),e.callbackPriority=t,e.callbackNode=n,t}return r!==null&&r!==null&&Ne(r),e.callbackPriority=2,e.callbackNode=null,2}function fd(e,t){if(su!==0&&su!==5)return e.callbackNode=null,e.callbackPriority=0,null;var n=e.callbackNode;if(Gu()&&e.callbackNode!==n)return null;var r=G;return r=nt(e,e===U?r:0,e.cancelPendingCommit!==null||e.timeoutHandle!==-1),r===0?null:(yu(e,r,t),dd(e,Ie()),e.callbackNode!=null&&e.callbackNode===n?fd.bind(null,e):null)}function pd(e,t){if(Gu())return null;yu(e,t,!0)}function md(){$d(function(){H&6?Me(Re,ld):ud()})}function hd(){if(od===0){var e=ya;e===0&&(e=Qe,Qe<<=1,!(Qe&261888)&&(Qe=256)),od=e}return od}function J(e){return e==null||typeof e==`symbol`||typeof e==`boolean`?null:typeof e==`function`?e:ln(``+e)}function gd(e,t){var n=t.ownerDocument.createElement(`input`);return n.name=t.name,n.value=t.value,e.id&&n.setAttribute(`form`,e.id),t.parentNode.insertBefore(n,t),e=new FormData(e),n.parentNode.removeChild(n),e}function _d(e,t,n,r,i){if(t===`submit`&&n&&n.stateNode===i){var a=J((i[vt]||null).action),o=r.submitter;o&&(t=(t=o[vt]||null)?J(t.formAction):o.getAttribute(`formAction`),t!==null&&(a=t,o=null));var s=new jn(`action`,`action`,null,r,i);e.push({event:s,listeners:[{instance:null,listener:function(){if(r.defaultPrevented){if(od!==0){var e=o?gd(i,o):new FormData(i);js(n,{pending:!0,data:e,method:i.method,action:a},null,e)}}else typeof a==`function`&&(s.preventDefault(),e=o?gd(i,o):new FormData(i),js(n,{pending:!0,data:e,method:i.method,action:a},a,e))},currentTarget:i}]})}}for(var vd=0;vd<ni.length;vd++){var yd=ni[vd];ri(yd.toLowerCase(),`on`+(yd[0].toUpperCase()+yd.slice(1)))}ri(Jr,`onAnimationEnd`),ri(Yr,`onAnimationIteration`),ri(Xr,`onAnimationStart`),ri(`dblclick`,`onDoubleClick`),ri(`focusin`,`onFocus`),ri(`focusout`,`onBlur`),ri(Zr,`onTransitionRun`),ri(Qr,`onTransitionStart`),ri($r,`onTransitionCancel`),ri(ei,`onTransitionEnd`),Pt(`onMouseEnter`,[`mouseout`,`mouseover`]),Pt(`onMouseLeave`,[`mouseout`,`mouseover`]),Pt(`onPointerEnter`,[`pointerout`,`pointerover`]),Pt(`onPointerLeave`,[`pointerout`,`pointerover`]),Nt(`onChange`,`change click focusin focusout input keydown keyup selectionchange`.split(` `)),Nt(`onSelect`,`focusout contextmenu dragend focusin keydown keyup mousedown mouseup selectionchange`.split(` `)),Nt(`onBeforeInput`,[`compositionend`,`keypress`,`textInput`,`paste`]),Nt(`onCompositionEnd`,`compositionend focusout keydown keypress keyup mousedown`.split(` `)),Nt(`onCompositionStart`,`compositionstart focusout keydown keypress keyup mousedown`.split(` `)),Nt(`onCompositionUpdate`,`compositionupdate focusout keydown keypress keyup mousedown`.split(` `));var bd=`abort canplay canplaythrough durationchange emptied encrypted ended error loadeddata loadedmetadata loadstart pause play playing progress ratechange resize seeked seeking stalled suspend timeupdate volumechange waiting`.split(` `),xd=new Set(`beforetoggle cancel close invalid load scroll scrollend toggle`.split(` `).concat(bd));function Sd(e,t){t=(t&4)!=0;for(var n=0;n<e.length;n++){var r=e[n],i=r.event;r=r.listeners;a:{var a=void 0;if(t)for(var o=r.length-1;0<=o;o--){var s=r[o],c=s.instance,l=s.currentTarget;if(s=s.listener,c!==a&&i.isPropagationStopped())break a;a=s,i.currentTarget=l;try{a(i)}catch(e){ii(e)}i.currentTarget=null,a=c}else for(o=0;o<r.length;o++){if(s=r[o],c=s.instance,l=s.currentTarget,s=s.listener,c!==a&&i.isPropagationStopped())break a;a=s,i.currentTarget=l;try{a(i)}catch(e){ii(e)}i.currentTarget=null,a=c}}}}function Y(e,t){var n=t[bt];n===void 0&&(n=t[bt]=new Set);var r=e+`__bubble`;n.has(r)||(Ed(t,e,2,!1),n.add(r))}function Cd(e,t,n){var r=0;t&&(r|=4),Ed(n,e,r,t)}var wd=`_reactListening`+Math.random().toString(36).slice(2);function Td(e){if(!e[wd]){e[wd]=!0,jt.forEach(function(t){t!==`selectionchange`&&(xd.has(t)||Cd(t,!1,e),Cd(t,!0,e))});var t=e.nodeType===9?e:e.ownerDocument;t===null||t[wd]||(t[wd]=!0,Cd(`selectionchange`,!1,t))}}function Ed(e,t,n,r){switch(_p(t)){case 2:var i=dp;break;case 8:i=fp;break;default:i=pp}n=i.bind(null,t,n,e),i=void 0,!bn||t!==`touchstart`&&t!==`touchmove`&&t!==`wheel`||(i=!0),r?i===void 0?e.addEventListener(t,n,!0):e.addEventListener(t,n,{capture:!0,passive:i}):i===void 0?e.addEventListener(t,n,!1):e.addEventListener(t,n,{passive:i})}function Dd(e,t,n,r,i){var a=r;if(!(t&1)&&!(t&2)&&r!==null)a:for(;;){if(r===null)return;var o=r.tag;if(o===3||o===4){var s=r.stateNode.containerInfo;if(s===i)break;if(o===4)for(o=r.return;o!==null;){var c=o.tag;if((c===3||c===4)&&o.stateNode.containerInfo===i)return;o=o.return}for(;s!==null;){if(o=Et(s),o===null)return;if(c=o.tag,c===5||c===6||c===26||c===27){r=a=o;continue a}s=s.parentNode}}r=r.return}_n(function(){var r=a,i=fn(n),o=[];a:{var s=ti.get(e);if(s!==void 0){var c=jn,u=e;switch(e){case`keypress`:if(En(n)===0)break a;case`keydown`:case`keyup`:c=Yn;break;case`focusin`:u=`focus`,c=Bn;break;case`focusout`:u=`blur`,c=Bn;break;case`beforeblur`:case`afterblur`:c=Bn;break;case`click`:if(n.button===2)break a;case`auxclick`:case`dblclick`:case`mousedown`:case`mousemove`:case`mouseup`:case`mouseout`:case`mouseover`:case`contextmenu`:c=Rn;break;case`drag`:case`dragend`:case`dragenter`:case`dragexit`:case`dragleave`:case`dragover`:case`dragstart`:case`drop`:c=zn;break;case`touchcancel`:case`touchend`:case`touchmove`:case`touchstart`:c=Zn;break;case Jr:case Yr:case Xr:c=Vn;break;case ei:c=Qn;break;case`scroll`:case`scrollend`:c=Nn;break;case`wheel`:c=$n;break;case`copy`:case`cut`:case`paste`:c=Hn;break;case`gotpointercapture`:case`lostpointercapture`:case`pointercancel`:case`pointerdown`:case`pointermove`:case`pointerout`:case`pointerover`:case`pointerup`:c=Xn;break;case`toggle`:case`beforetoggle`:c=er}var d=(t&4)!=0,f=!d&&(e===`scroll`||e===`scrollend`),p=d?s===null?null:s+`Capture`:s;d=[];for(var m=r,h;m!==null;){var g=m;if(h=g.stateNode,g=g.tag,g!==5&&g!==26&&g!==27||h===null||p===null||(g=vn(m,p),g!=null&&d.push(Od(m,g,h))),f)break;m=m.return}0<d.length&&(s=new c(s,u,null,n,i),o.push({event:s,listeners:d}))}}if(!(t&7)){a:{if(s=e===`mouseover`||e===`pointerover`,c=e===`mouseout`||e===`pointerout`,s&&n!==dn&&(u=n.relatedTarget||n.fromElement)&&(Et(u)||u[yt]))break a;if((c||s)&&(s=i.window===i?i:(s=i.ownerDocument)?s.defaultView||s.parentWindow:window,c?(u=n.relatedTarget||n.toElement,c=r,u=u?Et(u):null,u!==null&&(f=l(u),d=u.tag,u!==f||d!==5&&d!==27&&d!==6)&&(u=null)):(c=null,u=r),c!==u)){if(d=Rn,g=`onMouseLeave`,p=`onMouseEnter`,m=`mouse`,(e===`pointerout`||e===`pointerover`)&&(d=Xn,g=`onPointerLeave`,p=`onPointerEnter`,m=`pointer`),f=c==null?s:Ot(c),h=u==null?s:Ot(u),s=new d(g,m+`leave`,c,n,i),s.target=f,s.relatedTarget=h,g=null,Et(i)===r&&(d=new d(p,m+`enter`,u,n,i),d.target=h,d.relatedTarget=f,g=d),f=g,c&&u)b:{for(d=Ad,p=c,m=u,h=0,g=p;g;g=d(g))h++;g=0;for(var _=m;_;_=d(_))g++;for(;0<h-g;)p=d(p),h--;for(;0<g-h;)m=d(m),g--;for(;h--;){if(p===m||m!==null&&p===m.alternate){d=p;break b}p=d(p),m=d(m)}d=null}else d=null;c!==null&&jd(o,s,c,d,!1),u!==null&&f!==null&&jd(o,f,u,d,!0)}}a:{if(s=r?Ot(r):window,c=s.nodeName&&s.nodeName.toLowerCase(),c===`select`||c===`input`&&s.type===`file`)var v=br;else if(mr(s))if(xr)v=kr;else{v=Dr;var y=Er}else c=s.nodeName,!c||c.toLowerCase()!==`input`||s.type!==`checkbox`&&s.type!==`radio`?r&&on(r.elementType)&&(v=br):v=Or;if(v&&=v(e,r)){hr(o,v,n,i);break a}y&&y(e,s,r),e===`focusout`&&r&&s.type===`number`&&r.memoizedProps.value!=null&&Qt(s,`number`,s.value)}switch(y=r?Ot(r):window,e){case`focusin`:(mr(y)||y.contentEditable===`true`)&&(zr=y,Br=r,Vr=null);break;case`focusout`:Vr=Br=zr=null;break;case`mousedown`:Hr=!0;break;case`contextmenu`:case`mouseup`:case`dragend`:Hr=!1,Ur(o,n,i);break;case`selectionchange`:if(Rr)break;case`keydown`:case`keyup`:Ur(o,n,i)}var b;if(nr)b:{switch(e){case`compositionstart`:var x=`onCompositionStart`;break b;case`compositionend`:x=`onCompositionEnd`;break b;case`compositionupdate`:x=`onCompositionUpdate`;break b}x=void 0}else ur?cr(e,n)&&(x=`onCompositionEnd`):e===`keydown`&&n.keyCode===229&&(x=`onCompositionStart`);x&&(ar&&n.locale!==`ko`&&(ur||x!==`onCompositionStart`?x===`onCompositionEnd`&&ur&&(b=Tn()):(Sn=i,Cn=`value`in Sn?Sn.value:Sn.textContent,ur=!0)),y=kd(r,x),0<y.length&&(x=new Un(x,e,null,n,i),o.push({event:x,listeners:y}),b?x.data=b:(b=lr(n),b!==null&&(x.data=b)))),(b=ir?dr(e,n):fr(e,n))&&(x=kd(r,`onBeforeInput`),0<x.length&&(y=new Un(`onBeforeInput`,`beforeinput`,null,n,i),o.push({event:y,listeners:x}),y.data=b)),_d(o,e,r,n,i)}Sd(o,t)})}function Od(e,t,n){return{instance:e,listener:t,currentTarget:n}}function kd(e,t){for(var n=t+`Capture`,r=[];e!==null;){var i=e,a=i.stateNode;if(i=i.tag,i!==5&&i!==26&&i!==27||a===null||(i=vn(e,n),i!=null&&r.unshift(Od(e,i,a)),i=vn(e,t),i!=null&&r.push(Od(e,i,a))),e.tag===3)return r;e=e.return}return[]}function Ad(e){if(e===null)return null;do e=e.return;while(e&&e.tag!==5&&e.tag!==27);return e||null}function jd(e,t,n,r,i){for(var a=t._reactName,o=[];n!==null&&n!==r;){var s=n,c=s.alternate,l=s.stateNode;if(s=s.tag,c!==null&&c===r)break;s!==5&&s!==26&&s!==27||l===null||(c=l,i?(l=vn(n,a),l!=null&&o.unshift(Od(n,l,c))):i||(l=vn(n,a),l!=null&&o.push(Od(n,l,c)))),n=n.return}o.length!==0&&e.push({event:t,listeners:o})}var Md=/\r\n?/g,Nd=/\u0000|\uFFFD/g;function Pd(e){return(typeof e==`string`?e:``+e).replace(Md,`
`).replace(Nd,``)}function Fd(e,t){return t=Pd(t),Pd(e)===t}function Id(e,t,n,r,i,a){switch(n){case`children`:typeof r==`string`?t===`body`||t===`textarea`&&r===``||O(e,r):(typeof r==`number`||typeof r==`bigint`)&&t!==`body`&&O(e,``+r);break;case`className`:Bt(e,`class`,r);break;case`tabIndex`:Bt(e,`tabindex`,r);break;case`dir`:case`role`:case`viewBox`:case`width`:case`height`:Bt(e,n,r);break;case`style`:an(e,r,a);break;case`data`:if(t!==`object`){Bt(e,`data`,r);break}case`src`:case`href`:if(r===``&&(t!==`a`||n!==`href`)){e.removeAttribute(n);break}if(r==null||typeof r==`function`||typeof r==`symbol`||typeof r==`boolean`){e.removeAttribute(n);break}r=ln(``+r),e.setAttribute(n,r);break;case`action`:case`formAction`:if(typeof r==`function`){e.setAttribute(n,`javascript:throw new Error('A React form was unexpectedly submitted. If you called form.submit() manually, consider using form.requestSubmit() instead. If you\\'re trying to use event.stopPropagation() in a submit event handler, consider also calling event.preventDefault().')`);break}else typeof a==`function`&&(n===`formAction`?(t!==`input`&&Id(e,t,`name`,i.name,i,null),Id(e,t,`formEncType`,i.formEncType,i,null),Id(e,t,`formMethod`,i.formMethod,i,null),Id(e,t,`formTarget`,i.formTarget,i,null)):(Id(e,t,`encType`,i.encType,i,null),Id(e,t,`method`,i.method,i,null),Id(e,t,`target`,i.target,i,null)));if(r==null||typeof r==`symbol`||typeof r==`boolean`){e.removeAttribute(n);break}r=ln(``+r),e.setAttribute(n,r);break;case`onClick`:r!=null&&(e.onclick=un);break;case`onScroll`:r!=null&&Y(`scroll`,e);break;case`onScrollEnd`:r!=null&&Y(`scrollend`,e);break;case`dangerouslySetInnerHTML`:if(r!=null){if(typeof r!=`object`||!(`__html`in r))throw Error(s(61));if(n=r.__html,n!=null){if(i.children!=null)throw Error(s(60));e.innerHTML=n}}break;case`multiple`:e.multiple=r&&typeof r!=`function`&&typeof r!=`symbol`;break;case`muted`:e.muted=r&&typeof r!=`function`&&typeof r!=`symbol`;break;case`suppressContentEditableWarning`:case`suppressHydrationWarning`:case`defaultValue`:case`defaultChecked`:case`innerHTML`:case`ref`:break;case`autoFocus`:break;case`xlinkHref`:if(r==null||typeof r==`function`||typeof r==`boolean`||typeof r==`symbol`){e.removeAttribute(`xlink:href`);break}n=ln(``+r),e.setAttributeNS(`http://www.w3.org/1999/xlink`,`xlink:href`,n);break;case`contentEditable`:case`spellCheck`:case`draggable`:case`value`:case`autoReverse`:case`externalResourcesRequired`:case`focusable`:case`preserveAlpha`:r!=null&&typeof r!=`function`&&typeof r!=`symbol`?e.setAttribute(n,``+r):e.removeAttribute(n);break;case`inert`:case`allowFullScreen`:case`async`:case`autoPlay`:case`controls`:case`default`:case`defer`:case`disabled`:case`disablePictureInPicture`:case`disableRemotePlayback`:case`formNoValidate`:case`hidden`:case`loop`:case`noModule`:case`noValidate`:case`open`:case`playsInline`:case`readOnly`:case`required`:case`reversed`:case`scoped`:case`seamless`:case`itemScope`:r&&typeof r!=`function`&&typeof r!=`symbol`?e.setAttribute(n,``):e.removeAttribute(n);break;case`capture`:case`download`:!0===r?e.setAttribute(n,``):!1!==r&&r!=null&&typeof r!=`function`&&typeof r!=`symbol`?e.setAttribute(n,r):e.removeAttribute(n);break;case`cols`:case`rows`:case`size`:case`span`:r!=null&&typeof r!=`function`&&typeof r!=`symbol`&&!isNaN(r)&&1<=r?e.setAttribute(n,r):e.removeAttribute(n);break;case`rowSpan`:case`start`:r==null||typeof r==`function`||typeof r==`symbol`||isNaN(r)?e.removeAttribute(n):e.setAttribute(n,r);break;case`popover`:Y(`beforetoggle`,e),Y(`toggle`,e),zt(e,`popover`,r);break;case`xlinkActuate`:Vt(e,`http://www.w3.org/1999/xlink`,`xlink:actuate`,r);break;case`xlinkArcrole`:Vt(e,`http://www.w3.org/1999/xlink`,`xlink:arcrole`,r);break;case`xlinkRole`:Vt(e,`http://www.w3.org/1999/xlink`,`xlink:role`,r);break;case`xlinkShow`:Vt(e,`http://www.w3.org/1999/xlink`,`xlink:show`,r);break;case`xlinkTitle`:Vt(e,`http://www.w3.org/1999/xlink`,`xlink:title`,r);break;case`xlinkType`:Vt(e,`http://www.w3.org/1999/xlink`,`xlink:type`,r);break;case`xmlBase`:Vt(e,`http://www.w3.org/XML/1998/namespace`,`xml:base`,r);break;case`xmlLang`:Vt(e,`http://www.w3.org/XML/1998/namespace`,`xml:lang`,r);break;case`xmlSpace`:Vt(e,`http://www.w3.org/XML/1998/namespace`,`xml:space`,r);break;case`is`:zt(e,`is`,r);break;case`innerText`:case`textContent`:break;default:(!(2<n.length)||n[0]!==`o`&&n[0]!==`O`||n[1]!==`n`&&n[1]!==`N`)&&(n=sn.get(n)||n,zt(e,n,r))}}function Ld(e,t,n,r,i,a){switch(n){case`style`:an(e,r,a);break;case`dangerouslySetInnerHTML`:if(r!=null){if(typeof r!=`object`||!(`__html`in r))throw Error(s(61));if(n=r.__html,n!=null){if(i.children!=null)throw Error(s(60));e.innerHTML=n}}break;case`children`:typeof r==`string`?O(e,r):(typeof r==`number`||typeof r==`bigint`)&&O(e,``+r);break;case`onScroll`:r!=null&&Y(`scroll`,e);break;case`onScrollEnd`:r!=null&&Y(`scrollend`,e);break;case`onClick`:r!=null&&(e.onclick=un);break;case`suppressContentEditableWarning`:case`suppressHydrationWarning`:case`innerHTML`:case`ref`:break;case`innerText`:case`textContent`:break;default:if(!Mt.hasOwnProperty(n))a:{if(n[0]===`o`&&n[1]===`n`&&(i=n.endsWith(`Capture`),t=n.slice(2,i?n.length-7:void 0),a=e[vt]||null,a=a==null?null:a[n],typeof a==`function`&&e.removeEventListener(t,a,i),typeof r==`function`)){typeof a!=`function`&&a!==null&&(n in e?e[n]=null:e.hasAttribute(n)&&e.removeAttribute(n)),e.addEventListener(t,r,i);break a}n in e?e[n]=r:!0===r?e.setAttribute(n,``):zt(e,n,r)}}}function Rd(e,t,n){switch(t){case`div`:case`span`:case`svg`:case`path`:case`a`:case`g`:case`p`:case`li`:break;case`img`:Y(`error`,e),Y(`load`,e);var r=!1,i=!1,a;for(a in n)if(n.hasOwnProperty(a)){var o=n[a];if(o!=null)switch(a){case`src`:r=!0;break;case`srcSet`:i=!0;break;case`children`:case`dangerouslySetInnerHTML`:throw Error(s(137,t));default:Id(e,t,a,o,n,null)}}i&&Id(e,t,`srcSet`,n.srcSet,n,null),r&&Id(e,t,`src`,n.src,n,null);return;case`input`:Y(`invalid`,e);var c=a=o=i=null,l=null,u=null;for(r in n)if(n.hasOwnProperty(r)){var d=n[r];if(d!=null)switch(r){case`name`:i=d;break;case`type`:o=d;break;case`checked`:l=d;break;case`defaultChecked`:u=d;break;case`value`:a=d;break;case`defaultValue`:c=d;break;case`children`:case`dangerouslySetInnerHTML`:if(d!=null)throw Error(s(137,t));break;default:Id(e,t,r,d,n,null)}}Zt(e,a,c,l,u,o,i,!1);return;case`select`:for(i in Y(`invalid`,e),r=o=a=null,n)if(n.hasOwnProperty(i)&&(c=n[i],c!=null))switch(i){case`value`:a=c;break;case`defaultValue`:o=c;break;case`multiple`:r=c;default:Id(e,t,i,c,n,null)}t=a,n=o,e.multiple=!!r,t==null?n!=null&&$t(e,!!r,n,!0):$t(e,!!r,t,!1);return;case`textarea`:for(o in Y(`invalid`,e),a=i=r=null,n)if(n.hasOwnProperty(o)&&(c=n[o],c!=null))switch(o){case`value`:r=c;break;case`defaultValue`:i=c;break;case`children`:a=c;break;case`dangerouslySetInnerHTML`:if(c!=null)throw Error(s(91));break;default:Id(e,t,o,c,n,null)}tn(e,r,i,a);return;case`option`:for(l in n)if(n.hasOwnProperty(l)&&(r=n[l],r!=null))switch(l){case`selected`:e.selected=r&&typeof r!=`function`&&typeof r!=`symbol`;break;default:Id(e,t,l,r,n,null)}return;case`dialog`:Y(`beforetoggle`,e),Y(`toggle`,e),Y(`cancel`,e),Y(`close`,e);break;case`iframe`:case`object`:Y(`load`,e);break;case`video`:case`audio`:for(r=0;r<bd.length;r++)Y(bd[r],e);break;case`image`:Y(`error`,e),Y(`load`,e);break;case`details`:Y(`toggle`,e);break;case`embed`:case`source`:case`link`:Y(`error`,e),Y(`load`,e);case`area`:case`base`:case`br`:case`col`:case`hr`:case`keygen`:case`meta`:case`param`:case`track`:case`wbr`:case`menuitem`:for(u in n)if(n.hasOwnProperty(u)&&(r=n[u],r!=null))switch(u){case`children`:case`dangerouslySetInnerHTML`:throw Error(s(137,t));default:Id(e,t,u,r,n,null)}return;default:if(on(t)){for(d in n)n.hasOwnProperty(d)&&(r=n[d],r!==void 0&&Ld(e,t,d,r,n,void 0));return}}for(c in n)n.hasOwnProperty(c)&&(r=n[c],r!=null&&Id(e,t,c,r,n,null))}function zd(e,t,n,r){switch(t){case`div`:case`span`:case`svg`:case`path`:case`a`:case`g`:case`p`:case`li`:break;case`input`:var i=null,a=null,o=null,c=null,l=null,u=null,d=null;for(m in n){var f=n[m];if(n.hasOwnProperty(m)&&f!=null)switch(m){case`checked`:break;case`value`:break;case`defaultValue`:l=f;default:r.hasOwnProperty(m)||Id(e,t,m,null,r,f)}}for(var p in r){var m=r[p];if(f=n[p],r.hasOwnProperty(p)&&(m!=null||f!=null))switch(p){case`type`:a=m;break;case`name`:i=m;break;case`checked`:u=m;break;case`defaultChecked`:d=m;break;case`value`:o=m;break;case`defaultValue`:c=m;break;case`children`:case`dangerouslySetInnerHTML`:if(m!=null)throw Error(s(137,t));break;default:m!==f&&Id(e,t,p,m,r,f)}}Xt(e,o,c,l,u,d,a,i);return;case`select`:for(a in m=o=c=p=null,n)if(l=n[a],n.hasOwnProperty(a)&&l!=null)switch(a){case`value`:break;case`multiple`:m=l;default:r.hasOwnProperty(a)||Id(e,t,a,null,r,l)}for(i in r)if(a=r[i],l=n[i],r.hasOwnProperty(i)&&(a!=null||l!=null))switch(i){case`value`:p=a;break;case`defaultValue`:c=a;break;case`multiple`:o=a;default:a!==l&&Id(e,t,i,a,r,l)}t=c,n=o,r=m,p==null?!!r!=!!n&&(t==null?$t(e,!!n,n?[]:``,!1):$t(e,!!n,t,!0)):$t(e,!!n,p,!1);return;case`textarea`:for(c in m=p=null,n)if(i=n[c],n.hasOwnProperty(c)&&i!=null&&!r.hasOwnProperty(c))switch(c){case`value`:break;case`children`:break;default:Id(e,t,c,null,r,i)}for(o in r)if(i=r[o],a=n[o],r.hasOwnProperty(o)&&(i!=null||a!=null))switch(o){case`value`:p=i;break;case`defaultValue`:m=i;break;case`children`:break;case`dangerouslySetInnerHTML`:if(i!=null)throw Error(s(91));break;default:i!==a&&Id(e,t,o,i,r,a)}en(e,p,m);return;case`option`:for(var h in n)if(p=n[h],n.hasOwnProperty(h)&&p!=null&&!r.hasOwnProperty(h))switch(h){case`selected`:e.selected=!1;break;default:Id(e,t,h,null,r,p)}for(l in r)if(p=r[l],m=n[l],r.hasOwnProperty(l)&&p!==m&&(p!=null||m!=null))switch(l){case`selected`:e.selected=p&&typeof p!=`function`&&typeof p!=`symbol`;break;default:Id(e,t,l,p,r,m)}return;case`img`:case`link`:case`area`:case`base`:case`br`:case`col`:case`embed`:case`hr`:case`keygen`:case`meta`:case`param`:case`source`:case`track`:case`wbr`:case`menuitem`:for(var g in n)p=n[g],n.hasOwnProperty(g)&&p!=null&&!r.hasOwnProperty(g)&&Id(e,t,g,null,r,p);for(u in r)if(p=r[u],m=n[u],r.hasOwnProperty(u)&&p!==m&&(p!=null||m!=null))switch(u){case`children`:case`dangerouslySetInnerHTML`:if(p!=null)throw Error(s(137,t));break;default:Id(e,t,u,p,r,m)}return;default:if(on(t)){for(var _ in n)p=n[_],n.hasOwnProperty(_)&&p!==void 0&&!r.hasOwnProperty(_)&&Ld(e,t,_,void 0,r,p);for(d in r)p=r[d],m=n[d],!r.hasOwnProperty(d)||p===m||p===void 0&&m===void 0||Ld(e,t,d,p,r,m);return}}for(var v in n)p=n[v],n.hasOwnProperty(v)&&p!=null&&!r.hasOwnProperty(v)&&Id(e,t,v,null,r,p);for(f in r)p=r[f],m=n[f],!r.hasOwnProperty(f)||p===m||p==null&&m==null||Id(e,t,f,p,r,m)}function Bd(e){switch(e){case`css`:case`script`:case`font`:case`img`:case`image`:case`input`:case`link`:return!0;default:return!1}}function Vd(){if(typeof performance.getEntriesByType==`function`){for(var e=0,t=0,n=performance.getEntriesByType(`resource`),r=0;r<n.length;r++){var i=n[r],a=i.transferSize,o=i.initiatorType,s=i.duration;if(a&&s&&Bd(o)){for(o=0,s=i.responseEnd,r+=1;r<n.length;r++){var c=n[r],l=c.startTime;if(l>s)break;var u=c.transferSize,d=c.initiatorType;u&&Bd(d)&&(c=c.responseEnd,o+=u*(c<s?1:(s-l)/(c-l)))}if(--r,t+=8*(a+o)/(i.duration/1e3),e++,10<e)break}}if(0<e)return t/e/1e6}return navigator.connection&&(e=navigator.connection.downlink,typeof e==`number`)?e:5}var Hd=null,Ud=null;function Wd(e){return e.nodeType===9?e:e.ownerDocument}function Gd(e){switch(e){case`http://www.w3.org/2000/svg`:return 1;case`http://www.w3.org/1998/Math/MathML`:return 2;default:return 0}}function Kd(e,t){if(e===0)switch(t){case`svg`:return 1;case`math`:return 2;default:return 0}return e===1&&t===`foreignObject`?0:e}function qd(e,t){return e===`textarea`||e===`noscript`||typeof t.children==`string`||typeof t.children==`number`||typeof t.children==`bigint`||typeof t.dangerouslySetInnerHTML==`object`&&t.dangerouslySetInnerHTML!==null&&t.dangerouslySetInnerHTML.__html!=null}var Jd=null;function Yd(){var e=window.event;return e&&e.type===`popstate`?e===Jd?!1:(Jd=e,!0):(Jd=null,!1)}var Xd=typeof setTimeout==`function`?setTimeout:void 0,Zd=typeof clearTimeout==`function`?clearTimeout:void 0,Qd=typeof Promise==`function`?Promise:void 0,$d=typeof queueMicrotask==`function`?queueMicrotask:Qd===void 0?Xd:function(e){return Qd.resolve(null).then(e).catch(ef)};function ef(e){setTimeout(function(){throw e})}function tf(e){return e===`head`}function nf(e,t){var n=t,r=0;do{var i=n.nextSibling;if(e.removeChild(n),i&&i.nodeType===8)if(n=i.data,n===`/$`||n===`/&`){if(r===0){e.removeChild(i),$(t);return}r--}else if(n===`$`||n===`$?`||n===`$~`||n===`$!`||n===`&`)r++;else if(n===`html`)gf(e.ownerDocument.documentElement);else if(n===`head`){n=e.ownerDocument.head,gf(n);for(var a=n.firstChild;a;){var o=a.nextSibling,s=a.nodeName;a[wt]||s===`SCRIPT`||s===`STYLE`||s===`LINK`&&a.rel.toLowerCase()===`stylesheet`||n.removeChild(a),a=o}}else n===`body`&&gf(e.ownerDocument.body);n=i}while(n);$(t)}function rf(e,t){var n=e;e=0;do{var r=n.nextSibling;if(n.nodeType===1?t?(n._stashedDisplay=n.style.display,n.style.display=`none`):(n.style.display=n._stashedDisplay||``,n.getAttribute(`style`)===``&&n.removeAttribute(`style`)):n.nodeType===3&&(t?(n._stashedText=n.nodeValue,n.nodeValue=``):n.nodeValue=n._stashedText||``),r&&r.nodeType===8)if(n=r.data,n===`/$`){if(e===0)break;e--}else n!==`$`&&n!==`$?`&&n!==`$~`&&n!==`$!`||e++;n=r}while(n)}function af(e){var t=e.firstChild;for(t&&t.nodeType===10&&(t=t.nextSibling);t;){var n=t;switch(t=t.nextSibling,n.nodeName){case`HTML`:case`HEAD`:case`BODY`:af(n),Tt(n);continue;case`SCRIPT`:case`STYLE`:continue;case`LINK`:if(n.rel.toLowerCase()===`stylesheet`)continue}e.removeChild(n)}}function X(e,t,n,r){for(;e.nodeType===1;){var i=n;if(e.nodeName.toLowerCase()!==t.toLowerCase()){if(!r&&(e.nodeName!==`INPUT`||e.type!==`hidden`))break}else if(!r)if(t===`input`&&e.type===`hidden`){var a=i.name==null?null:``+i.name;if(i.type===`hidden`&&e.getAttribute(`name`)===a)return e}else return e;else if(!e[wt])switch(t){case`meta`:if(!e.hasAttribute(`itemprop`))break;return e;case`link`:if(a=e.getAttribute(`rel`),a===`stylesheet`&&e.hasAttribute(`data-precedence`)||a!==i.rel||e.getAttribute(`href`)!==(i.href==null||i.href===``?null:i.href)||e.getAttribute(`crossorigin`)!==(i.crossOrigin==null?null:i.crossOrigin)||e.getAttribute(`title`)!==(i.title==null?null:i.title))break;return e;case`style`:if(e.hasAttribute(`data-precedence`))break;return e;case`script`:if(a=e.getAttribute(`src`),(a!==(i.src==null?null:i.src)||e.getAttribute(`type`)!==(i.type==null?null:i.type)||e.getAttribute(`crossorigin`)!==(i.crossOrigin==null?null:i.crossOrigin))&&a&&e.hasAttribute(`async`)&&!e.hasAttribute(`itemprop`))break;return e;default:return e}if(e=df(e.nextSibling),e===null)break}return null}function of(e,t,n){if(t===``)return null;for(;e.nodeType!==3;)if((e.nodeType!==1||e.nodeName!==`INPUT`||e.type!==`hidden`)&&!n||(e=df(e.nextSibling),e===null))return null;return e}function sf(e,t){for(;e.nodeType!==8;)if((e.nodeType!==1||e.nodeName!==`INPUT`||e.type!==`hidden`)&&!t||(e=df(e.nextSibling),e===null))return null;return e}function cf(e){return e.data===`$?`||e.data===`$~`}function lf(e){return e.data===`$!`||e.data===`$?`&&e.ownerDocument.readyState!==`loading`}function uf(e,t){var n=e.ownerDocument;if(e.data===`$~`)e._reactRetry=t;else if(e.data!==`$?`||n.readyState!==`loading`)t();else{var r=function(){t(),n.removeEventListener(`DOMContentLoaded`,r)};n.addEventListener(`DOMContentLoaded`,r),e._reactRetry=r}}function df(e){for(;e!=null;e=e.nextSibling){var t=e.nodeType;if(t===1||t===3)break;if(t===8){if(t=e.data,t===`$`||t===`$!`||t===`$?`||t===`$~`||t===`&`||t===`F!`||t===`F`)break;if(t===`/$`||t===`/&`)return null}}return e}var ff=null;function pf(e){e=e.nextSibling;for(var t=0;e;){if(e.nodeType===8){var n=e.data;if(n===`/$`||n===`/&`){if(t===0)return df(e.nextSibling);t--}else n!==`$`&&n!==`$!`&&n!==`$?`&&n!==`$~`&&n!==`&`||t++}e=e.nextSibling}return null}function mf(e){e=e.previousSibling;for(var t=0;e;){if(e.nodeType===8){var n=e.data;if(n===`$`||n===`$!`||n===`$?`||n===`$~`||n===`&`){if(t===0)return e;t--}else n!==`/$`&&n!==`/&`||t++}e=e.previousSibling}return null}function hf(e,t,n){switch(t=Wd(n),e){case`html`:if(e=t.documentElement,!e)throw Error(s(452));return e;case`head`:if(e=t.head,!e)throw Error(s(453));return e;case`body`:if(e=t.body,!e)throw Error(s(454));return e;default:throw Error(s(451))}}function gf(e){for(var t=e.attributes;t.length;)e.removeAttributeNode(t[0]);Tt(e)}var _f=new Map,vf=new Set;function yf(e){return typeof e.getRootNode==`function`?e.getRootNode():e.nodeType===9?e:e.ownerDocument}var bf=D.d;D.d={f:xf,r:Sf,D:Tf,C:Ef,L:Df,m:Of,X:Af,S:kf,M:jf};function xf(){var e=bf.f(),t=Cu();return e||t}function Sf(e){var t=Dt(e);t!==null&&t.tag===5&&t.type===`form`?Ns(t):bf.r(e)}var Cf=typeof document>`u`?null:document;function wf(e,t,n){var r=Cf;if(r&&typeof t==`string`&&t){var i=Yt(t);i=`link[rel="`+e+`"][href="`+i+`"]`,typeof n==`string`&&(i+=`[crossorigin="`+n+`"]`),vf.has(i)||(vf.add(i),e={rel:e,crossOrigin:n,href:t},r.querySelector(i)===null&&(t=r.createElement(`link`),Rd(t,`link`,e),At(t),r.head.appendChild(t)))}}function Tf(e){bf.D(e),wf(`dns-prefetch`,e,null)}function Ef(e,t){bf.C(e,t),wf(`preconnect`,e,t)}function Df(e,t,n){bf.L(e,t,n);var r=Cf;if(r&&e&&t){var i=`link[rel="preload"][as="`+Yt(t)+`"]`;t===`image`&&n&&n.imageSrcSet?(i+=`[imagesrcset="`+Yt(n.imageSrcSet)+`"]`,typeof n.imageSizes==`string`&&(i+=`[imagesizes="`+Yt(n.imageSizes)+`"]`)):i+=`[href="`+Yt(e)+`"]`;var a=i;switch(t){case`style`:a=Nf(e);break;case`script`:a=Lf(e)}_f.has(a)||(e=h({rel:`preload`,href:t===`image`&&n&&n.imageSrcSet?void 0:e,as:t},n),_f.set(a,e),r.querySelector(i)!==null||t===`style`&&r.querySelector(Pf(a))||t===`script`&&r.querySelector(Rf(a))||(t=r.createElement(`link`),Rd(t,`link`,e),At(t),r.head.appendChild(t)))}}function Of(e,t){bf.m(e,t);var n=Cf;if(n&&e){var r=t&&typeof t.as==`string`?t.as:`script`,i=`link[rel="modulepreload"][as="`+Yt(r)+`"][href="`+Yt(e)+`"]`,a=i;switch(r){case`audioworklet`:case`paintworklet`:case`serviceworker`:case`sharedworker`:case`worker`:case`script`:a=Lf(e)}if(!_f.has(a)&&(e=h({rel:`modulepreload`,href:e},t),_f.set(a,e),n.querySelector(i)===null)){switch(r){case`audioworklet`:case`paintworklet`:case`serviceworker`:case`sharedworker`:case`worker`:case`script`:if(n.querySelector(Rf(a)))return}r=n.createElement(`link`),Rd(r,`link`,e),At(r),n.head.appendChild(r)}}}function kf(e,t,n){bf.S(e,t,n);var r=Cf;if(r&&e){var i=kt(r).hoistableStyles,a=Nf(e);t||=`default`;var o=i.get(a);if(!o){var s={loading:0,preload:null};if(o=r.querySelector(Pf(a)))s.loading=5;else{e=h({rel:`stylesheet`,href:e,"data-precedence":t},n),(n=_f.get(a))&&Vf(e,n);var c=o=r.createElement(`link`);At(c),Rd(c,`link`,e),c._p=new Promise(function(e,t){c.onload=e,c.onerror=t}),c.addEventListener(`load`,function(){s.loading|=1}),c.addEventListener(`error`,function(){s.loading|=2}),s.loading|=4,Bf(o,t,r)}o={type:`stylesheet`,instance:o,count:1,state:s},i.set(a,o)}}}function Af(e,t){bf.X(e,t);var n=Cf;if(n&&e){var r=kt(n).hoistableScripts,i=Lf(e),a=r.get(i);a||(a=n.querySelector(Rf(i)),a||(e=h({src:e,async:!0},t),(t=_f.get(i))&&Hf(e,t),a=n.createElement(`script`),At(a),Rd(a,`link`,e),n.head.appendChild(a)),a={type:`script`,instance:a,count:1,state:null},r.set(i,a))}}function jf(e,t){bf.M(e,t);var n=Cf;if(n&&e){var r=kt(n).hoistableScripts,i=Lf(e),a=r.get(i);a||(a=n.querySelector(Rf(i)),a||(e=h({src:e,async:!0,type:`module`},t),(t=_f.get(i))&&Hf(e,t),a=n.createElement(`script`),At(a),Rd(a,`link`,e),n.head.appendChild(a)),a={type:`script`,instance:a,count:1,state:null},r.set(i,a))}}function Mf(e,t,n,r){var i=(i=ve.current)?yf(i):null;if(!i)throw Error(s(446));switch(e){case`meta`:case`title`:return null;case`style`:return typeof n.precedence==`string`&&typeof n.href==`string`?(t=Nf(n.href),n=kt(i).hoistableStyles,r=n.get(t),r||(r={type:`style`,instance:null,count:0,state:null},n.set(t,r)),r):{type:`void`,instance:null,count:0,state:null};case`link`:if(n.rel===`stylesheet`&&typeof n.href==`string`&&typeof n.precedence==`string`){e=Nf(n.href);var a=kt(i).hoistableStyles,o=a.get(e);if(o||(i=i.ownerDocument||i,o={type:`stylesheet`,instance:null,count:0,state:{loading:0,preload:null}},a.set(e,o),(a=i.querySelector(Pf(e)))&&!a._p&&(o.instance=a,o.state.loading=5),_f.has(e)||(n={rel:`preload`,as:`style`,href:n.href,crossOrigin:n.crossOrigin,integrity:n.integrity,media:n.media,hrefLang:n.hrefLang,referrerPolicy:n.referrerPolicy},_f.set(e,n),a||If(i,e,n,o.state))),t&&r===null)throw Error(s(528,``));return o}if(t&&r!==null)throw Error(s(529,``));return null;case`script`:return t=n.async,n=n.src,typeof n==`string`&&t&&typeof t!=`function`&&typeof t!=`symbol`?(t=Lf(n),n=kt(i).hoistableScripts,r=n.get(t),r||(r={type:`script`,instance:null,count:0,state:null},n.set(t,r)),r):{type:`void`,instance:null,count:0,state:null};default:throw Error(s(444,e))}}function Nf(e){return`href="`+Yt(e)+`"`}function Pf(e){return`link[rel="stylesheet"][`+e+`]`}function Ff(e){return h({},e,{"data-precedence":e.precedence,precedence:null})}function If(e,t,n,r){e.querySelector(`link[rel="preload"][as="style"][`+t+`]`)?r.loading=1:(t=e.createElement(`link`),r.preload=t,t.addEventListener(`load`,function(){return r.loading|=1}),t.addEventListener(`error`,function(){return r.loading|=2}),Rd(t,`link`,n),At(t),e.head.appendChild(t))}function Lf(e){return`[src="`+Yt(e)+`"]`}function Rf(e){return`script[async]`+e}function zf(e,t,n){if(t.count++,t.instance===null)switch(t.type){case`style`:var r=e.querySelector(`style[data-href~="`+Yt(n.href)+`"]`);if(r)return t.instance=r,At(r),r;var i=h({},n,{"data-href":n.href,"data-precedence":n.precedence,href:null,precedence:null});return r=(e.ownerDocument||e).createElement(`style`),At(r),Rd(r,`style`,i),Bf(r,n.precedence,e),t.instance=r;case`stylesheet`:i=Nf(n.href);var a=e.querySelector(Pf(i));if(a)return t.state.loading|=4,t.instance=a,At(a),a;r=Ff(n),(i=_f.get(i))&&Vf(r,i),a=(e.ownerDocument||e).createElement(`link`),At(a);var o=a;return o._p=new Promise(function(e,t){o.onload=e,o.onerror=t}),Rd(a,`link`,r),t.state.loading|=4,Bf(a,n.precedence,e),t.instance=a;case`script`:return a=Lf(n.src),(i=e.querySelector(Rf(a)))?(t.instance=i,At(i),i):(r=n,(i=_f.get(a))&&(r=h({},n),Hf(r,i)),e=e.ownerDocument||e,i=e.createElement(`script`),At(i),Rd(i,`link`,r),e.head.appendChild(i),t.instance=i);case`void`:return null;default:throw Error(s(443,t.type))}else t.type===`stylesheet`&&!(t.state.loading&4)&&(r=t.instance,t.state.loading|=4,Bf(r,n.precedence,e));return t.instance}function Bf(e,t,n){for(var r=n.querySelectorAll(`link[rel="stylesheet"][data-precedence],style[data-precedence]`),i=r.length?r[r.length-1]:null,a=i,o=0;o<r.length;o++){var s=r[o];if(s.dataset.precedence===t)a=s;else if(a!==i)break}a?a.parentNode.insertBefore(e,a.nextSibling):(t=n.nodeType===9?n.head:n,t.insertBefore(e,t.firstChild))}function Vf(e,t){e.crossOrigin??=t.crossOrigin,e.referrerPolicy??=t.referrerPolicy,e.title??=t.title}function Hf(e,t){e.crossOrigin??=t.crossOrigin,e.referrerPolicy??=t.referrerPolicy,e.integrity??=t.integrity}var Uf=null;function Wf(e,t,n){if(Uf===null){var r=new Map,i=Uf=new Map;i.set(n,r)}else i=Uf,r=i.get(n),r||(r=new Map,i.set(n,r));if(r.has(e))return r;for(r.set(e,null),n=n.getElementsByTagName(e),i=0;i<n.length;i++){var a=n[i];if(!(a[wt]||a[_t]||e===`link`&&a.getAttribute(`rel`)===`stylesheet`)&&a.namespaceURI!==`http://www.w3.org/2000/svg`){var o=a.getAttribute(t)||``;o=e+o;var s=r.get(o);s?s.push(a):r.set(o,[a])}}return r}function Gf(e,t,n){e=e.ownerDocument||e,e.head.insertBefore(n,t===`title`?e.querySelector(`head > title`):null)}function Kf(e,t,n){if(n===1||t.itemProp!=null)return!1;switch(e){case`meta`:case`title`:return!0;case`style`:if(typeof t.precedence!=`string`||typeof t.href!=`string`||t.href===``)break;return!0;case`link`:if(typeof t.rel!=`string`||typeof t.href!=`string`||t.href===``||t.onLoad||t.onError)break;switch(t.rel){case`stylesheet`:return e=t.disabled,typeof t.precedence==`string`&&e==null;default:return!0}case`script`:if(t.async&&typeof t.async!=`function`&&typeof t.async!=`symbol`&&!t.onLoad&&!t.onError&&t.src&&typeof t.src==`string`)return!0}return!1}function qf(e){return!(e.type===`stylesheet`&&!(e.state.loading&3))}function Jf(e,t,n,r){if(n.type===`stylesheet`&&(typeof r.media!=`string`||!1!==matchMedia(r.media).matches)&&!(n.state.loading&4)){if(n.instance===null){var i=Nf(r.href),a=t.querySelector(Pf(i));if(a){t=a._p,typeof t==`object`&&t&&typeof t.then==`function`&&(e.count++,e=Zf.bind(e),t.then(e,e)),n.state.loading|=4,n.instance=a,At(a);return}a=t.ownerDocument||t,r=Ff(r),(i=_f.get(i))&&Vf(r,i),a=a.createElement(`link`),At(a);var o=a;o._p=new Promise(function(e,t){o.onload=e,o.onerror=t}),Rd(a,`link`,r),n.instance=a}e.stylesheets===null&&(e.stylesheets=new Map),e.stylesheets.set(n,t),(t=n.state.preload)&&!(n.state.loading&3)&&(e.count++,n=Zf.bind(e),t.addEventListener(`load`,n),t.addEventListener(`error`,n))}}var Yf=0;function Xf(e,t){return e.stylesheets&&e.count===0&&$f(e,e.stylesheets),0<e.count||0<e.imgCount?function(n){var r=setTimeout(function(){if(e.stylesheets&&$f(e,e.stylesheets),e.unsuspend){var t=e.unsuspend;e.unsuspend=null,t()}},6e4+t);0<e.imgBytes&&Yf===0&&(Yf=62500*Vd());var i=setTimeout(function(){if(e.waitingForImages=!1,e.count===0&&(e.stylesheets&&$f(e,e.stylesheets),e.unsuspend)){var t=e.unsuspend;e.unsuspend=null,t()}},(e.imgBytes>Yf?50:800)+t);return e.unsuspend=n,function(){e.unsuspend=null,clearTimeout(r),clearTimeout(i)}}:null}function Zf(){if(this.count--,this.count===0&&(this.imgCount===0||!this.waitingForImages)){if(this.stylesheets)$f(this,this.stylesheets);else if(this.unsuspend){var e=this.unsuspend;this.unsuspend=null,e()}}}var Qf=null;function $f(e,t){e.stylesheets=null,e.unsuspend!==null&&(e.count++,Qf=new Map,t.forEach(ep,e),Qf=null,Zf.call(e))}function ep(e,t){if(!(t.state.loading&4)){var n=Qf.get(e);if(n)var r=n.get(null);else{n=new Map,Qf.set(e,n);for(var i=e.querySelectorAll(`link[data-precedence],style[data-precedence]`),a=0;a<i.length;a++){var o=i[a];(o.nodeName===`LINK`||o.getAttribute(`media`)!==`not all`)&&(n.set(o.dataset.precedence,o),r=o)}r&&n.set(null,r)}i=t.instance,o=i.getAttribute(`data-precedence`),a=n.get(o)||r,a===r&&n.set(null,i),n.set(o,i),this.count++,r=Zf.bind(this),i.addEventListener(`load`,r),i.addEventListener(`error`,r),a?a.parentNode.insertBefore(i,a.nextSibling):(e=e.nodeType===9?e.head:e,e.insertBefore(i,e.firstChild)),t.state.loading|=4}}var tp={$$typeof:S,Provider:null,Consumer:null,_currentValue:ue,_currentValue2:ue,_threadCount:0};function np(e,t,n,r,i,a,o,s,c){this.tag=1,this.containerInfo=e,this.pingCache=this.current=this.pendingChildren=null,this.timeoutHandle=-1,this.callbackNode=this.next=this.pendingContext=this.context=this.cancelPendingCommit=null,this.callbackPriority=0,this.expirationTimes=ot(-1),this.entangledLanes=this.shellSuspendCounter=this.errorRecoveryDisabledLanes=this.expiredLanes=this.warmLanes=this.pingedLanes=this.suspendedLanes=this.pendingLanes=0,this.entanglements=ot(0),this.hiddenUpdates=ot(null),this.identifierPrefix=r,this.onUncaughtError=i,this.onCaughtError=a,this.onRecoverableError=o,this.pooledCache=null,this.pooledCacheLanes=0,this.formState=c,this.incompleteTransitions=new Map}function rp(e,t,n,r,i,a,o,s,c,l,u,d){return e=new np(e,t,n,o,c,l,u,d,s),t=1,!0===a&&(t|=24),a=gi(3,null,null,t),e.current=a,a.stateNode=e,t=ha(),t.refCount++,e.pooledCache=t,t.refCount++,a.memoizedState={element:r,isDehydrated:n,cache:t},Ja(a),e}function ip(e){return e?(e=mi,e):mi}function ap(e,t,n,r,i,a){i=ip(i),r.context===null?r.context=i:r.pendingContext=i,r=Xa(t),r.payload={element:n},a=a===void 0?null:a,a!==null&&(r.callback=a),n=Za(e,r,t),n!==null&&(vu(n,e,t),Qa(n,e,t))}function op(e,t){if(e=e.memoizedState,e!==null&&e.dehydrated!==null){var n=e.retryLane;e.retryLane=n!==0&&n<t?n:t}}function sp(e,t){op(e,t),(e=e.alternate)&&op(e,t)}function cp(e){if(e.tag===13||e.tag===31){var t=di(e,67108864);t!==null&&vu(t,e,67108864),sp(e,67108864)}}function lp(e){if(e.tag===13||e.tag===31){var t=gu();t=ft(t);var n=di(e,t);n!==null&&vu(n,e,t),sp(e,t)}}var up=!0;function dp(e,t,n,r){var i=E.T;E.T=null;var a=D.p;try{D.p=2,pp(e,t,n,r)}finally{D.p=a,E.T=i}}function fp(e,t,n,r){var i=E.T;E.T=null;var a=D.p;try{D.p=8,pp(e,t,n,r)}finally{D.p=a,E.T=i}}function pp(e,t,n,r){if(up){var i=mp(r);if(i===null)Dd(e,t,r,hp,n),wp(e,r);else if(Ep(i,e,t,n,r))r.stopPropagation();else if(wp(e,r),t&4&&-1<Cp.indexOf(e)){for(;i!==null;){var a=Dt(i);if(a!==null)switch(a.tag){case 3:if(a=a.stateNode,a.current.memoizedState.isDehydrated){var o=tt(a.pendingLanes);if(o!==0){var s=a;for(s.pendingLanes|=2,s.entangledLanes|=2;o;){var c=1<<31-Je(o);s.entanglements[1]|=c,o&=~c}sd(a),!(H&6)&&(au=Ie()+500,cd(0,!1))}}break;case 31:case 13:s=di(a,2),s!==null&&vu(s,a,2),Cu(),sp(a,2)}if(a=mp(r),a===null&&Dd(e,t,r,hp,n),a===i)break;i=a}i!==null&&r.stopPropagation()}else Dd(e,t,r,null,n)}}function mp(e){return e=fn(e),gp(e)}var hp=null;function gp(e){if(hp=null,e=Et(e),e!==null){var t=l(e);if(t===null)e=null;else{var n=t.tag;if(n===13){if(e=u(t),e!==null)return e;e=null}else if(n===31){if(e=d(t),e!==null)return e;e=null}else if(n===3){if(t.stateNode.current.memoizedState.isDehydrated)return t.tag===3?t.stateNode.containerInfo:null;e=null}else t!==e&&(e=null)}}return hp=e,null}function _p(e){switch(e){case`beforetoggle`:case`cancel`:case`click`:case`close`:case`contextmenu`:case`copy`:case`cut`:case`auxclick`:case`dblclick`:case`dragend`:case`dragstart`:case`drop`:case`focusin`:case`focusout`:case`input`:case`invalid`:case`keydown`:case`keypress`:case`keyup`:case`mousedown`:case`mouseup`:case`paste`:case`pause`:case`play`:case`pointercancel`:case`pointerdown`:case`pointerup`:case`ratechange`:case`reset`:case`resize`:case`seeked`:case`submit`:case`toggle`:case`touchcancel`:case`touchend`:case`touchstart`:case`volumechange`:case`change`:case`selectionchange`:case`textInput`:case`compositionstart`:case`compositionend`:case`compositionupdate`:case`beforeblur`:case`afterblur`:case`beforeinput`:case`blur`:case`fullscreenchange`:case`focus`:case`hashchange`:case`popstate`:case`select`:case`selectstart`:return 2;case`drag`:case`dragenter`:case`dragexit`:case`dragleave`:case`dragover`:case`mousemove`:case`mouseout`:case`mouseover`:case`pointermove`:case`pointerout`:case`pointerover`:case`scroll`:case`touchmove`:case`wheel`:case`mouseenter`:case`mouseleave`:case`pointerenter`:case`pointerleave`:return 8;case`message`:switch(Le()){case Re:return 2;case ze:return 8;case Be:case Ve:return 32;case He:return 268435456;default:return 32}default:return 32}}var vp=!1,yp=null,bp=null,xp=null,Sp=new Map,Z=new Map,Q=[],Cp=`mousedown mouseup touchcancel touchend touchstart auxclick dblclick pointercancel pointerdown pointerup dragend dragstart drop compositionend compositionstart keydown keypress keyup input textInput copy cut paste click change contextmenu reset`.split(` `);function wp(e,t){switch(e){case`focusin`:case`focusout`:yp=null;break;case`dragenter`:case`dragleave`:bp=null;break;case`mouseover`:case`mouseout`:xp=null;break;case`pointerover`:case`pointerout`:Sp.delete(t.pointerId);break;case`gotpointercapture`:case`lostpointercapture`:Z.delete(t.pointerId)}}function Tp(e,t,n,r,i,a){return e===null||e.nativeEvent!==a?(e={blockedOn:t,domEventName:n,eventSystemFlags:r,nativeEvent:a,targetContainers:[i]},t!==null&&(t=Dt(t),t!==null&&cp(t)),e):(e.eventSystemFlags|=r,t=e.targetContainers,i!==null&&t.indexOf(i)===-1&&t.push(i),e)}function Ep(e,t,n,r,i){switch(t){case`focusin`:return yp=Tp(yp,e,t,n,r,i),!0;case`dragenter`:return bp=Tp(bp,e,t,n,r,i),!0;case`mouseover`:return xp=Tp(xp,e,t,n,r,i),!0;case`pointerover`:var a=i.pointerId;return Sp.set(a,Tp(Sp.get(a)||null,e,t,n,r,i)),!0;case`gotpointercapture`:return a=i.pointerId,Z.set(a,Tp(Z.get(a)||null,e,t,n,r,i)),!0}return!1}function Dp(e){var t=Et(e.target);if(t!==null){var n=l(t);if(n!==null){if(t=n.tag,t===13){if(t=u(n),t!==null){e.blockedOn=t,ht(e.priority,function(){lp(n)});return}}else if(t===31){if(t=d(n),t!==null){e.blockedOn=t,ht(e.priority,function(){lp(n)});return}}else if(t===3&&n.stateNode.current.memoizedState.isDehydrated){e.blockedOn=n.tag===3?n.stateNode.containerInfo:null;return}}}e.blockedOn=null}function Op(e){if(e.blockedOn!==null)return!1;for(var t=e.targetContainers;0<t.length;){var n=mp(e.nativeEvent);if(n===null){n=e.nativeEvent;var r=new n.constructor(n.type,n);dn=r,n.target.dispatchEvent(r),dn=null}else return t=Dt(n),t!==null&&cp(t),e.blockedOn=n,!1;t.shift()}return!0}function kp(e,t,n){Op(e)&&n.delete(t)}function Ap(){vp=!1,yp!==null&&Op(yp)&&(yp=null),bp!==null&&Op(bp)&&(bp=null),xp!==null&&Op(xp)&&(xp=null),Sp.forEach(kp),Z.forEach(kp)}function jp(e,n){e.blockedOn===n&&(e.blockedOn=null,vp||(vp=!0,t.unstable_scheduleCallback(t.unstable_NormalPriority,Ap)))}var Mp=null;function Np(e){Mp!==e&&(Mp=e,t.unstable_scheduleCallback(t.unstable_NormalPriority,function(){Mp===e&&(Mp=null);for(var t=0;t<e.length;t+=3){var n=e[t],r=e[t+1],i=e[t+2];if(typeof r!=`function`){if(gp(r||n)===null)continue;break}var a=Dt(n);a!==null&&(e.splice(t,3),t-=3,js(a,{pending:!0,data:i,method:n.method,action:r},r,i))}}))}function $(e){function t(t){return jp(t,e)}yp!==null&&jp(yp,e),bp!==null&&jp(bp,e),xp!==null&&jp(xp,e),Sp.forEach(t),Z.forEach(t);for(var n=0;n<Q.length;n++){var r=Q[n];r.blockedOn===e&&(r.blockedOn=null)}for(;0<Q.length&&(n=Q[0],n.blockedOn===null);)Dp(n),n.blockedOn===null&&Q.shift();if(n=(e.ownerDocument||e).$$reactFormReplay,n!=null)for(r=0;r<n.length;r+=3){var i=n[r],a=n[r+1],o=i[vt]||null;if(typeof a==`function`)o||Np(n);else if(o){var s=null;if(a&&a.hasAttribute(`formAction`)){if(i=a,o=a[vt]||null)s=o.formAction;else if(gp(i)!==null)continue}else s=o.action;typeof s==`function`?n[r+1]=s:(n.splice(r,3),r-=3),Np(n)}}}function Pp(){function e(e){e.canIntercept&&e.info===`react-transition`&&e.intercept({handler:function(){return new Promise(function(e){return i=e})},focusReset:`manual`,scroll:`manual`})}function t(){i!==null&&(i(),i=null),r||setTimeout(n,20)}function n(){if(!r&&!navigation.transition){var e=navigation.currentEntry;e&&e.url!=null&&navigation.navigate(e.url,{state:e.getState(),info:`react-transition`,history:`replace`})}}if(typeof navigation==`object`){var r=!1,i=null;return navigation.addEventListener(`navigate`,e),navigation.addEventListener(`navigatesuccess`,t),navigation.addEventListener(`navigateerror`,t),setTimeout(n,100),function(){r=!0,navigation.removeEventListener(`navigate`,e),navigation.removeEventListener(`navigatesuccess`,t),navigation.removeEventListener(`navigateerror`,t),i!==null&&(i(),i=null)}}}function Fp(e){this._internalRoot=e}Ip.prototype.render=Fp.prototype.render=function(e){var t=this._internalRoot;if(t===null)throw Error(s(409));var n=t.current;ap(n,gu(),e,t,null,null)},Ip.prototype.unmount=Fp.prototype.unmount=function(){var e=this._internalRoot;if(e!==null){this._internalRoot=null;var t=e.containerInfo;ap(e.current,2,null,e,null,null),Cu(),t[yt]=null}};function Ip(e){this._internalRoot=e}Ip.prototype.unstable_scheduleHydration=function(e){if(e){var t=mt();e={blockedOn:null,target:e,priority:t};for(var n=0;n<Q.length&&t!==0&&t<Q[n].priority;n++);Q.splice(n,0,e),n===0&&Dp(e)}};var Lp=r.version;if(Lp!==`19.2.7`)throw Error(s(527,Lp,`19.2.7`));D.findDOMNode=function(e){var t=e._reactInternals;if(t===void 0)throw typeof e.render==`function`?Error(s(188)):(e=Object.keys(e).join(`,`),Error(s(268,e)));return e=p(t),e=e===null?null:m(e),e=e===null?null:e.stateNode,e};var Rp={bundleType:0,version:`19.2.7`,rendererPackageName:`react-dom`,currentDispatcherRef:E,reconcilerVersion:`19.2.7`};if(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__<`u`){var zp=__REACT_DEVTOOLS_GLOBAL_HOOK__;if(!zp.isDisabled&&zp.supportsFiber)try{Ge=zp.inject(Rp),Ke=zp}catch{}}e.createRoot=function(e,t){if(!c(e))throw Error(s(299));var n=!1,r=``,i=ec,a=tc,o=nc;return t!=null&&(!0===t.unstable_strictMode&&(n=!0),t.identifierPrefix!==void 0&&(r=t.identifierPrefix),t.onUncaughtError!==void 0&&(i=t.onUncaughtError),t.onCaughtError!==void 0&&(a=t.onCaughtError),t.onRecoverableError!==void 0&&(o=t.onRecoverableError)),t=rp(e,1,!1,null,null,n,r,null,i,a,o,Pp),e[yt]=t.current,Td(e),new Fp(t)}})),c=e(((e,t)=>{function n(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>`u`||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!=`function`))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(n)}catch(e){console.error(e)}}n(),t.exports=s()})),l=n(),u=c(),d=[{kind:`linear`,label:`Linear`,summary:`Passes the signal through unchanged; useful for regression outputs.`},{kind:`relu`,label:`ReLU`,summary:`Clips negative values to zero and keeps positive values, creating sparse activations.`},{kind:`leakyRelu`,label:`Leaky ReLU`,summary:`Keeps a small negative slope so negative inputs do not become completely silent.`},{kind:`sigmoid`,label:`Sigmoid`,summary:`Squashes values into 0 to 1, which is useful for probability-style outputs.`},{kind:`tanh`,label:`Tanh`,summary:`Squashes values into -1 to 1 and stays centered around zero.`},{kind:`softplus`,label:`Softplus`,summary:`A smooth ReLU-like curve that never has a sharp corner.`}];function f(e,t){switch(t){case`linear`:return e;case`relu`:return Math.max(0,e);case`leakyRelu`:return e>=0?e:e*.1;case`sigmoid`:return 1/(1+Math.exp(-e));case`tanh`:return Math.tanh(e);case`softplus`:return Math.log1p(Math.exp(-Math.abs(e)))+Math.max(e,0)}}function p(e){return d.find(t=>t.kind===e)??d[0]}var m=[{id:`red`,label:`red`,embedding:[1,0]},{id:`blue`,label:`blue`,embedding:[0,1]},{id:`purple`,label:`purple`,embedding:[1,1]}],h=[[1,0],[0,1]],g=[[1,1],[-1,1]],_=[[2,0],[0,1]];function v(e){return Math.abs(e)<1e-12?0:e}function y(e,t){if(e.length===0||t.length!==e.length||t.some(e=>e.length!==t[0].length)||![...e,...t.flat()].every(Number.isFinite))throw Error(`NN12 V1 needs finite row vectors and compatible rectangular matrices.`);return t[0].map((n,r)=>v(e.reduce((e,n,i)=>e+n*t[i][r],0)))}function b(e=m,t=h,n=g,r=_){if(e.length!==3||new Set(e.map(e=>e.id)).size!==e.length||e.some(e=>e.label.length===0||e.embedding.length!==2)||[t,n,r].some(e=>e.length!==2||e.some(e=>e.length!==2)))throw Error(`NN12 V1 needs three unique two-number tokens and three 2 x 2 matrices.`);let i=e.map(e=>({id:e.id,label:e.label,embedding:[...e.embedding],query:y(e.embedding,t),key:y(e.embedding,n),value:y(e.embedding,r)})),a=i[0].key.length,o=Math.sqrt(a),s=i.flatMap(e=>i.map(t=>{let n=e.query.map((e,n)=>v(e*t.key[n])),r=v(n.reduce((e,t)=>e+t,0));return{queryId:e.id,keyId:t.id,products:n,rawScore:r,scaledScore:v(r/o)}}));return{projections:i,dotProducts:s,rawScoreMatrix:i.map(e=>i.map(t=>s.find(n=>n.queryId===e.id&&n.keyId===t.id).rawScore)),scaledScoreMatrix:i.map(e=>i.map(t=>s.find(n=>n.queryId===e.id&&n.keyId===t.id).scaledScore)),scaleDivisor:o}}function x(e,t,n){let r=e.dotProducts.find(e=>e.queryId===t&&e.keyId===n);if(r===void 0)throw Error(`Unknown attention cell ${t} -> ${n}.`);return r}var ee=b(),S=ee.projections.map(e=>e.id),C=ee.scaledScoreMatrix,te=ee.projections.map(e=>e.value);function ne(e){return Math.abs(e)<1e-12?0:e}function w(e,t,n){return e.length===t&&e.every(e=>e.length===n&&e.every(Number.isFinite))}function re(e=!0,t=C,n=te,r=S){if(r.length!==3||new Set(r).size!==r.length||!w(t,3,3)||!w(n,3,2))throw Error(`NN13 V1 needs three token IDs, a finite 3 x 3 score matrix, and finite 3 x 2 values.`);let i=t.map((t,i)=>{let a=t.map((t,n)=>!e||n<=i),o=t.map((e,t)=>a[t]?e:null),s=Math.max(...o.filter(e=>e!==null)),c=o.map(e=>e===null?null:ne(e-s)),l=c.map(e=>e===null?0:Math.exp(e)),u=l.reduce((e,t)=>e+t,0),d=l.map(e=>ne(e/u)),f=n.map((e,t)=>e.map(e=>ne(d[t]*e))),p=n[0].map((e,t)=>ne(f.reduce((e,n)=>e+n[t],0)));return{queryId:r[i],allowed:a,scaledScores:[...t],maskedScores:o,rowMax:s,shiftedScores:c,exponentials:l,denominator:u,weights:d,values:n.map(e=>[...e]),valueContributions:f,context:p}});return{causal:e,tokenIds:[...r],rows:i,weightMatrix:i.map(e=>e.weights),contextMatrix:i.map(e=>e.context)}}function ie(e,t){let n=e.rows.find(e=>e.queryId===t);if(n===void 0)throw Error(`Unknown attention softmax query ${t}.`);return n}var ae=e((e=>{var t=Symbol.for(`react.transitional.element`),n=Symbol.for(`react.fragment`);function r(e,n,r){var i=null;if(r!==void 0&&(i=``+r),n.key!==void 0&&(i=``+n.key),`key`in n)for(var a in r={},n)a!==`key`&&(r[a]=n[a]);else r=n;return n=r.ref,{$$typeof:t,type:e,key:i,ref:n===void 0?null:n,props:r}}e.Fragment=n,e.jsx=r,e.jsxs=r})),T=e(((e,t)=>{t.exports=ae()}))();function oe(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function se(e){return`[${e.map(oe).join(`, `)}]`}function ce(e){return`[${e.map(e=>e===null?`blocked`:oe(e)).join(`, `)}]`}function le({onShowMultiHead:e,onShowScores:t}){let[n,r]=(0,l.useState)(!0),[i,a]=(0,l.useState)(`blue`),o=(0,l.useMemo)(()=>re(n),[n]),s=ie(o,i);return(0,T.jsxs)(`main`,{className:`workspace workspace--attention-softmax`,children:[(0,T.jsxs)(`section`,{className:`attention-softmax-stage`,"aria-label":`Causal attention weight trace`,children:[(0,T.jsxs)(`div`,{className:`attention-softmax-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN13 · normalize without looking ahead`}),(0,T.jsx)(`h2`,{children:`Causal-softmax mixer`}),(0,T.jsx)(`p`,{children:`Mask future keys, normalize one query row into weights, then follow each weight into the value vector it scales.`})]}),(0,T.jsx)(`div`,{className:`attention-softmax-chip`,children:n?`causal decoder`:`full context`})]}),(0,T.jsxs)(`section`,{className:`attention-weight-panel`,"aria-label":`Attention weight matrix`,children:[(0,T.jsxs)(`div`,{className:`attention-softmax-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Rows normalize independently`}),(0,T.jsxs)(`h2`,{children:[n?`Causal`:`Unmasked`,` attention weights`]})]}),(0,T.jsx)(`code`,{children:`each row sums to 1`})]}),(0,T.jsxs)(`div`,{className:`attention-weight-grid`,role:`grid`,"aria-label":`${n?`Causal`:`Unmasked`} attention weight matrix`,children:[(0,T.jsx)(`span`,{className:`attention-grid-corner`,children:`q \\ k`}),o.tokenIds.map(e=>(0,T.jsxs)(`span`,{className:`attention-grid-label`,children:[e,` k`]},`weight-key-${e}`)),o.rows.flatMap(e=>[(0,T.jsxs)(`button`,{"aria-label":`Select ${e.queryId} query row`,"aria-pressed":i===e.queryId,className:i===e.queryId?`attention-weight-row-button attention-weight-row-button--active`:`attention-weight-row-button`,type:`button`,onClick:()=>a(e.queryId),children:[e.queryId,` q`]},`weight-query-${e.queryId}`),...e.weights.map((t,n)=>{let r=!e.allowed[n];return(0,T.jsxs)(`div`,{"aria-label":`${e.queryId} query to ${o.tokenIds[n]} key: ${r?`blocked`:oe(t)}`,className:r?`attention-weight-cell attention-weight-cell--blocked`:i===e.queryId?`attention-weight-cell attention-weight-cell--selected-row`:`attention-weight-cell`,role:`gridcell`,children:[(0,T.jsx)(`strong`,{children:r?`blocked`:oe(t)}),(0,T.jsx)(`span`,{"aria-hidden":`true`,style:{width:`${Math.max(t*100,0)}%`}})]},`${e.queryId}-${o.tokenIds[n]}`)})])]})]}),(0,T.jsxs)(`section`,{className:`attention-normalize-panel`,"aria-label":`Selected softmax row trace`,children:[(0,T.jsxs)(`div`,{className:`attention-softmax-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected · `,s.queryId,` query`]}),(0,T.jsx)(`h2`,{children:`Score → mask → stable exponentials → weights`})]}),(0,T.jsxs)(`code`,{children:[`max = `,oe(s.rowMax)]})]}),(0,T.jsxs)(`div`,{className:`attention-normalize-flow`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`scaled scores`}),(0,T.jsx)(`code`,{children:se(s.scaledScores)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`after mask`}),(0,T.jsx)(`code`,{children:ce(s.maskedScores)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`subtract max, exp`}),(0,T.jsx)(`code`,{children:se(s.exponentials)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,T.jsxs)(`div`,{className:`attention-normalize-flow__result`,children:[(0,T.jsxs)(`small`,{children:[`divide by `,oe(s.denominator)]}),(0,T.jsx)(`code`,{children:se(s.weights)})]})]})]}),(0,T.jsxs)(`section`,{className:`attention-value-mix-panel`,"aria-label":`Selected weighted value mix`,children:[(0,T.jsxs)(`div`,{className:`attention-softmax-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Weights finally meet values`}),(0,T.jsxs)(`h2`,{children:[`Build the `,s.queryId,` context`]})]}),(0,T.jsxs)(`div`,{className:`attention-context-result`,children:[(0,T.jsx)(`small`,{children:`context`}),(0,T.jsx)(`strong`,{children:se(s.context)})]})]}),(0,T.jsx)(`div`,{className:`attention-value-lanes`,children:o.tokenIds.map((e,t)=>(0,T.jsxs)(`div`,{className:s.allowed[t]?`attention-value-lane`:`attention-value-lane attention-value-lane--blocked`,children:[(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`attention-token-dot attention-token-dot--${e}`}),e,` value`]}),(0,T.jsxs)(`code`,{children:[oe(s.weights[t]),` × `,se(s.values[t])]}),(0,T.jsxs)(`strong`,{children:[`= `,se(s.valueContributions[t])]})]},e))})]})]}),(0,T.jsxs)(`aside`,{className:`attention-softmax-controls`,"aria-label":`Causal attention controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One information boundary`}),(0,T.jsx)(`h2`,{children:`Mask controls`}),(0,T.jsx)(`p`,{children:`Select a whole query row. Softmax belongs to that row, not to one score cell.`}),(0,T.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:t,children:`Return to Q/K/V scores`}),(0,T.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Open multi-head add and norm`}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:n,onChange:e=>r(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Block future keys`}),(0,T.jsx)(`small`,{children:`Allow column j only when j ≤ query row i.`})]})]}),(0,T.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Query row selection`,children:o.tokenIds.map(e=>(0,T.jsx)(`button`,{"aria-pressed":i===e,type:`button`,onClick:()=>a(e),children:e},e))}),(0,T.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected context`}),(0,T.jsx)(`strong`,{children:se(s.context)}),(0,T.jsxs)(`span`,{children:[s.queryId,` reads `,s.allowed.filter(Boolean).length,` value`,s.allowed.filter(Boolean).length===1?``:`s`,`.`]})]}),(0,T.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,T.jsx)(`span`,{children:`Why subtract the maximum?`}),(0,T.jsx)(`p`,{children:`It keeps exponentials finite without changing their normalized proportions. The maximum shifted score is always zero.`})]}),(0,T.jsxs)(`div`,{className:`attention-next-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Multiple heads repeat this calculation with different projections, then concatenate their context vectors.`})]})]})]})}var E=[`red`,`blue`,`purple`],D=[`red`,`blue`,`purple`],ue=[[1,0],[0,1]],de=[[1,0,-1],[0,1,-1]],fe=[0,0,0],pe=.5;function me(e){return Math.abs(e)<1e-12?0:e}function he(e,t,n){return e.length===t&&e.every(e=>e.length===n&&e.every(Number.isFinite))}function ge(e,t,n,r){let i=E.map((t,r)=>e.map((e,t)=>me(e*n[t][r]))),a=i.map((e,t)=>me(e.reduce((e,t)=>e+t,0)+r[t])),o=Math.max(...a),s=a.map(e=>me(e-o)),c=s.map(Math.exp),l=c.reduce((e,t)=>e+t,0),u=c.map(e=>e/l),d=u[t];return{logitProducts:i,logits:a,rowMax:o,shiftedLogits:s,exponentials:c,denominator:l,probabilities:u,targetProbability:d,loss:-Math.log(d)}}function _e(e,t,n,r){return e.reduce((e,i,a)=>e+ge(i,t[a],n,r).loss,0)/e.length}function ve(e=pe,t=ue,n=de,r=fe){if(!Number.isFinite(e)||e<=0||!he(t,2,2)||!he(n,2,3)||r.length!==3||!r.every(Number.isFinite))throw Error(`NN15 V1 needs two 2D decoder states, a 2 x 3 unembedding, three finite biases, and a positive learning rate.`);let i=D.slice(0,-1),a=D.slice(1),o=Array.from({length:2},()=>[0,0,0]),s=[0,0,0],c=t.map((e,c)=>{let l=i[c],u=a[c],d=E.indexOf(u),f=ge(e,d,n,r),p=f.probabilities.map((e,n)=>(e-+(n===d))/t.length),m=e.map(e=>p.map(t=>me(e*t)));for(let e=0;e<2;e+=1)for(let t=0;t<3;t+=1)o[e][t]+=m[e][t];for(let e=0;e<3;e+=1)s[e]+=p[e];let h=e.map((e,t)=>me(p.reduce((e,r,i)=>e+r*n[t][i],0)));return{position:c,inputToken:l,targetToken:u,targetIndex:d,causalPrefix:D.slice(0,c+1),decoderState:[...e],...f,logitGradients:p,unembeddingGradientContribution:m,biasGradientContribution:[...p],stateGradient:h}}),l=n.map((t,n)=>t.map((t,r)=>t-e*o[n][r])),u=r.map((t,n)=>t-e*s[n]),d=1e-6,f=c.map(e=>e.targetIndex),p=Array.from({length:2},()=>[0,0,0]);for(let e=0;e<2;e+=1)for(let i=0;i<3;i+=1){let a=n.map(e=>[...e]),o=n.map(e=>[...e]);a[e][i]+=d,o[e][i]-=d,p[e][i]=(_e(t,f,a,r)-_e(t,f,o,r))/(2*d)}let m=r.map((e,i)=>{let a=[...r],o=[...r];return a[i]+=d,o[i]-=d,(_e(t,f,n,a)-_e(t,f,n,o))/(2*d)}),h=[...o.flatMap((e,t)=>e.map((e,n)=>Math.abs(e-p[t][n]))),...s.map((e,t)=>Math.abs(e-m[t]))],g=c.map(e=>{let t=ge(e.decoderState,e.targetIndex,l,u);return{position:e.position,logits:t.logits,probabilities:t.probabilities,targetProbability:t.targetProbability,loss:t.loss}});return{vocabulary:[...E],sequence:[...D],learningRate:e,rows:c,meanLoss:c.reduce((e,t)=>e+t.loss,0)/c.length,unembeddingGradient:o,biasGradient:s,gradientCheck:{epsilon:d,numericalUnembeddingGradient:p,numericalBiasGradient:m,maxAbsoluteError:Math.max(...h)},updatedUnembedding:l,updatedBias:u,postUpdateRows:g,postUpdateMeanLoss:g.reduce((e,t)=>e+t.loss,0)/g.length}}function ye(e,t){let n=e.rows[t];if(n===void 0)throw Error(`Unknown decoder training position ${t}.`);return n}function be(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function xe(e){return`[${e.map(be).join(`, `)}]`}function Se(e){return e.map(xe).join(`  `)}function Ce({onShowMultiHead:e}){let t=(0,l.useMemo)(()=>ve(),[]),[n,r]=(0,l.useState)(1),[i,a]=(0,l.useState)(!1),o=ye(t,n),s=i?t.postUpdateRows[n]:o;return(0,T.jsxs)(`main`,{className:`workspace workspace--decoder`,children:[(0,T.jsxs)(`section`,{className:`decoder-stage`,"aria-label":`Tiny decoder language model training trace`,children:[(0,T.jsxs)(`div`,{className:`decoder-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN15 - a complete next-token learning step`}),(0,T.jsx)(`h2`,{children:`Tiny decoder training trace`}),(0,T.jsx)(`p`,{children:`Shift one sequence into two causal predictions, turn saved decoder states into vocabulary probabilities, then follow the shared error through cross-entropy and one loss-reducing SGD update.`})]}),(0,T.jsx)(`div`,{className:`decoder-chip`,children:`3-token vocabulary - 2 positions`})]}),(0,T.jsxs)(`section`,{className:`decoder-shift-panel`,"aria-label":`Causal next-token sequence shift`,children:[(0,T.jsxs)(`div`,{className:`decoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One sequence - shifted by one`}),(0,T.jsx)(`h2`,{children:`Prefixes predict what comes next`})]}),(0,T.jsx)(`code`,{children:`red blue purple`})]}),(0,T.jsx)(`div`,{className:`decoder-position-lanes`,children:t.rows.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Select position ${e.position}: ${e.causalPrefix.join(` `)} predicts ${e.targetToken}`,"aria-pressed":n===e.position,className:`decoder-position-button`,type:`button`,onClick:()=>r(e.position),children:[(0,T.jsxs)(`span`,{children:[`position `,e.position]}),(0,T.jsx)(`strong`,{children:e.causalPrefix.join(` `)}),(0,T.jsx)(`i`,{"aria-hidden":`true`,children:`->`}),(0,T.jsx)(`strong`,{children:e.targetToken}),(0,T.jsx)(`small`,{children:`future target stays outside the prefix`})]},e.position))})]}),(0,T.jsxs)(`section`,{className:`decoder-prediction-panel`,"aria-label":`Selected decoder prediction at position ${n}`,children:[(0,T.jsxs)(`div`,{className:`decoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected - position `,n]}),(0,T.jsx)(`h2`,{children:i?`Rerun the updated head`:`State to target surprise`})]}),(0,T.jsxs)(`div`,{className:`decoder-loss-badge`,children:[(0,T.jsx)(`small`,{children:`position loss`}),(0,T.jsx)(`strong`,{children:be(s.loss)})]})]}),(0,T.jsxs)(`div`,{className:`decoder-forward-flow`,children:[(0,T.jsxs)(`div`,{className:`decoder-state-node`,children:[(0,T.jsx)(`small`,{children:`saved causal state`}),(0,T.jsxs)(`strong`,{children:[`h_`,o.inputToken]}),(0,T.jsx)(`code`,{children:xe(o.decoderState)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`decoder-logit-node`,children:[(0,T.jsx)(`small`,{children:i?`updated logits`:`shared head logits`}),(0,T.jsx)(`code`,{children:xe(s.logits)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`decoder-probability-node`,children:[(0,T.jsx)(`small`,{children:`stable softmax`}),(0,T.jsx)(`code`,{children:xe(s.probabilities)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`decoder-target-node`,children:[(0,T.jsx)(`small`,{children:`target probability`}),(0,T.jsxs)(`strong`,{children:[`P(`,o.targetToken,`) = `,be(s.targetProbability)]}),(0,T.jsxs)(`code`,{children:[`-ln(P) = `,be(s.loss)]})]})]}),(0,T.jsx)(`div`,{className:`decoder-vocabulary-grid`,role:`list`,"aria-label":`Vocabulary probability distribution`,children:t.vocabulary.map((e,t)=>{let n=s.probabilities[t],r=t===o.targetIndex;return(0,T.jsxs)(`div`,{className:r?`decoder-vocabulary-row decoder-vocabulary-row--target`:`decoder-vocabulary-row`,role:`listitem`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`span`,{children:[e,r?` - target`:``]}),(0,T.jsx)(`strong`,{children:be(n)})]}),(0,T.jsx)(`i`,{"aria-hidden":`true`,style:{width:`${n*100}%`}}),i?null:(0,T.jsxs)(`code`,{children:[be(o.logitProducts[t][0]),` + `,be(o.logitProducts[t][1]),` + bias = `,be(o.logits[t])]})]},e)})}),i?null:(0,T.jsxs)(`div`,{className:`decoder-softmax-trace`,"aria-label":`Stable softmax arithmetic`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`row max`}),(0,T.jsx)(`code`,{children:be(o.rowMax)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`shift logits`}),(0,T.jsx)(`code`,{children:xe(o.shiftedLogits)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`exponentials`}),(0,T.jsx)(`code`,{children:xe(o.exponentials)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`denominator`}),(0,T.jsx)(`code`,{children:be(o.denominator)})]})]})]}),(0,T.jsxs)(`section`,{className:`decoder-gradient-panel`,"aria-label":`Decoder loss gradient trace`,children:[(0,T.jsxs)(`div`,{className:`decoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Probability minus target - divided by two`}),(0,T.jsx)(`h2`,{children:`Error flows back through the shared head`})]}),(0,T.jsx)(`code`,{children:`(p - one_hot) / positions`})]}),(0,T.jsxs)(`div`,{className:`decoder-gradient-flow`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`logit gradient`}),(0,T.jsx)(`code`,{children:xe(o.logitGradients)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`this position's unembedding contribution`}),(0,T.jsx)(`code`,{children:Se(o.unembeddingGradientContribution)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias contribution`}),(0,T.jsx)(`code`,{children:xe(o.biasGradientContribution)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`decoder-state-gradient`,children:[(0,T.jsx)(`small`,{children:`gradient entering decoder body`}),(0,T.jsx)(`code`,{children:xe(o.stateGradient)})]})]})]}),(0,T.jsxs)(`section`,{className:`decoder-update-panel`,"aria-label":`Shared decoder head SGD update`,children:[(0,T.jsxs)(`div`,{className:`decoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Both positions reduce into one update`}),(0,T.jsx)(`h2`,{children:`Shared-head SGD checkpoint`})]}),(0,T.jsxs)(`code`,{children:[`parameter - `,t.learningRate,` x gradient`]})]}),(0,T.jsxs)(`div`,{className:`decoder-update-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`unembedding before`}),(0,T.jsx)(`code`,{children:Se(de)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`reduced gradient`}),(0,T.jsx)(`code`,{children:Se(t.unembeddingGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`unembedding after`}),(0,T.jsx)(`code`,{children:Se(t.updatedUnembedding)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias before`}),(0,T.jsx)(`code`,{children:xe(fe)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias gradient`}),(0,T.jsx)(`code`,{children:xe(t.biasGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias after`}),(0,T.jsx)(`code`,{children:xe(t.updatedBias)})]})]}),(0,T.jsxs)(`div`,{className:`decoder-gradient-audit`,children:[(0,T.jsx)(`span`,{children:`Central finite-difference audit`}),(0,T.jsxs)(`code`,{children:[`epsilon = `,t.gradientCheck.epsilon]}),(0,T.jsxs)(`strong`,{children:[`max error `,t.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,T.jsxs)(`div`,{className:`decoder-loss-drop`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean loss before`}),(0,T.jsx)(`strong`,{children:be(t.meanLoss)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean loss after one step`}),(0,T.jsx)(`strong`,{children:be(t.postUpdateMeanLoss)})]}),(0,T.jsx)(`p`,{children:`Both target probabilities rise; the deterministic objective falls.`})]})]})]}),(0,T.jsxs)(`aside`,{className:`decoder-controls`,"aria-label":`Tiny decoder training controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Inspect one prediction`}),(0,T.jsx)(`h2`,{children:`Training controls`}),(0,T.jsx)(`p`,{children:`The causal prefixes and saved states do not change. The toggle swaps only the shared vocabulary head before and after its one SGD step.`}),(0,T.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Return to multi-head block`}),(0,T.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Decoder position selection`,children:t.rows.map(e=>(0,T.jsxs)(`button`,{"aria-pressed":n===e.position,type:`button`,onClick:()=>r(e.position),children:[`position `,e.position]},e.position))}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:i,onChange:e=>a(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Use updated vocabulary head`}),(0,T.jsx)(`small`,{children:`Rerun logits and loss after one SGD step.`})]})]}),(0,T.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected target`}),(0,T.jsx)(`strong`,{children:o.targetToken}),(0,T.jsxs)(`span`,{children:[o.causalPrefix.join(` `),` -> `,o.targetToken]})]}),(0,T.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,T.jsx)(`span`,{children:`Frozen on purpose`}),(0,T.jsx)(`p`,{children:`This first trace updates unembedding and bias. The state gradient is preserved for a later full-decoder autograd pass.`})]}),(0,T.jsxs)(`div`,{className:`attention-next-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Add token sampling and a generation trace, then continue the saved state gradients through every decoder-block parameter.`})]})]})]})}var we=[`red`,`blue`,`purple`],Te=[[2,0],[0,1],[2,1]],Ee=[{id:`horizontal`,queryProjection:[.5,0],keyProjection:[.5,0],valueProjection:[1,0]},{id:`vertical`,queryProjection:[0,1],keyProjection:[0,1],valueProjection:[0,1]}],De=[[1,0],[0,1]],Oe={epsilon:1e-5,gamma:[1,1],beta:[0,0]};function ke(e){return Math.abs(e)<1e-12?0:e}function Ae(e,t,n){return e.length===t&&e.every(e=>e.length===n&&e.every(Number.isFinite))}function je(e,t){return e.map((e,n)=>ke(e*t[n]))}function Me(e,t,n){let r=je(e[t],n.queryProjection),i=ke(r.reduce((e,t)=>e+t,0)),a=e.map(e=>je(e,n.keyProjection)),o=a.map(e=>ke(e.reduce((e,t)=>e+t,0))),s=e.map(e=>je(e,n.valueProjection)),c=s.map(e=>ke(e.reduce((e,t)=>e+t,0))),l=o.map(e=>ke(i*e/1)),u=l.map((e,n)=>n<=t),d=l.map((e,t)=>u[t]?e:null),f=Math.max(...d.filter(e=>e!==null)),p=d.map(e=>e===null?null:ke(e-f)),m=p.map(e=>e===null?0:Math.exp(e)),h=m.reduce((e,t)=>e+t,0),g=m.map(e=>ke(e/h)),_=g.map((e,t)=>ke(e*c[t]));return{id:n.id,queryProducts:r,query:i,keyProducts:a,keys:o,valueProducts:s,values:c,scaleDivisor:1,scaledScores:l,allowed:u,maskedScores:d,rowMax:f,shiftedScores:p,exponentials:m,denominator:h,weights:g,valueContributions:_,context:ke(_.reduce((e,t)=>e+t,0))}}function Ne(e=!0,t=!0,n=Te,r=we,i=Ee,a=De,o=Oe.epsilon,s=Oe.gamma,c=Oe.beta){if(r.length!==3||new Set(r).size!==3||!Ae(n,3,2)||i.length!==2||new Set(i.map(e=>e.id)).size!==2||i.some(e=>!Ae([e.queryProjection,e.keyProjection,e.valueProjection],3,2))||!Ae(a,2,2)||!Number.isFinite(o)||o<=0||!Ae([s,c],2,2))throw Error(`NN14 V1 needs three 2D tokens, two scalar heads, a 2 x 2 output projection, and finite layer-norm parameters.`);let l=n.map((l,u)=>{let d=i.map(e=>Me(n,u,e)),f=d.map(e=>e.context),p=a[0].map((e,t)=>f.map((e,n)=>ke(e*a[n][t]))),m=p.map(e=>ke(e.reduce((e,t)=>e+t,0))),h=m.map((t,n)=>ke(t+(e?l[n]:0))),g=h.reduce((e,t)=>e+t,0)/2,_=h.map(e=>ke(e-g)),v=_.map(e=>e*e),y=v.reduce((e,t)=>e+t,0)/2,b=Math.sqrt(y+o),x=_.map(e=>ke(e/b)),ee=x.map((e,t)=>ke(e*s[t])),S=ee.map((e,t)=>ke(e+c[t]));return{tokenId:r[u],input:[...l],heads:d,concatenated:f,outputProjectionProducts:p,projectedAttention:m,residualSum:h,layerNorm:{mean:g,centered:_,squaredDeviations:v,variance:y,denominator:b,normalized:x,affineProducts:ee,output:S},output:t?S:h}});return{includeResidual:e,applyLayerNorm:t,tokenIds:[...r],rows:l}}function Pe(e,t){let n=e.rows.find(e=>e.tokenId===t);if(n===void 0)throw Error(`Unknown multi-head attention token ${t}.`);return n}function Fe(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function Ie(e){return`[${e.map(Fe).join(`, `)}]`}function Le(e){return e===`horizontal`?`Head A - horizontal`:`Head B - vertical`}function Re({onShowDecoder:e,onShowWeights:t}){let[n,r]=(0,l.useState)(`blue`),[i,a]=(0,l.useState)(!0),[o,s]=(0,l.useState)(!0),c=(0,l.useMemo)(()=>Ne(i,o),[o,i]),u=Pe(c,n);return(0,T.jsxs)(`main`,{className:`workspace workspace--multi-head`,children:[(0,T.jsxs)(`section`,{className:`multi-head-stage`,"aria-label":`Multi-head attention block trace`,children:[(0,T.jsxs)(`div`,{className:`multi-head-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN14 - parallel views rejoin one stream`}),(0,T.jsx)(`h2`,{children:`Multi-head add-and-norm block`}),(0,T.jsx)(`p`,{children:`Run two causal heads on the same token, keep their weights separate, then follow concatenation, projection, residual, and layer normalization without skipping a boundary.`})]}),(0,T.jsx)(`div`,{className:`multi-head-chip`,children:`2 heads x 1 feature`})]}),(0,T.jsxs)(`section`,{className:`multi-head-panel`,"aria-label":`Two attention heads for ${n}`,children:[(0,T.jsxs)(`div`,{className:`multi-head-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected - `,n,` query`]}),(0,T.jsx)(`h2`,{children:`Same token, different learned views`})]}),(0,T.jsx)(`code`,{children:`each head softmaxes alone`})]}),(0,T.jsx)(`div`,{className:`multi-head-lanes`,children:u.heads.map(e=>(0,T.jsxs)(`article`,{className:`multi-head-lane multi-head-lane--${e.id}`,children:[(0,T.jsxs)(`div`,{className:`multi-head-lane__heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:Le(e.id)}),(0,T.jsxs)(`strong`,{children:[`q = `,Fe(e.query)]})]}),(0,T.jsxs)(`code`,{children:[`context `,Fe(e.context)]})]}),(0,T.jsxs)(`div`,{className:`multi-head-score-row`,children:[(0,T.jsx)(`span`,{children:`scores`}),(0,T.jsx)(`code`,{children:Ie(e.scaledScores)})]}),(0,T.jsx)(`div`,{className:`multi-head-weight-row`,role:`list`,"aria-label":`${e.id} weights`,children:c.tokenIds.map((t,n)=>(0,T.jsxs)(`div`,{className:e.allowed[n]?`multi-head-weight`:`multi-head-weight multi-head-weight--blocked`,role:`listitem`,children:[(0,T.jsx)(`span`,{children:t}),(0,T.jsx)(`strong`,{children:e.allowed[n]?Fe(e.weights[n]):`blocked`}),(0,T.jsx)(`i`,{"aria-hidden":`true`,style:{width:`${e.weights[n]*100}%`}})]},t))}),(0,T.jsx)(`div`,{className:`multi-head-value-row`,children:c.tokenIds.map((t,n)=>(0,T.jsxs)(`code`,{children:[Fe(e.weights[n]),` x `,Fe(e.values[n]),` = `,Fe(e.valueContributions[n])]},t))})]},e.id))})]}),(0,T.jsxs)(`section`,{className:`multi-head-join-panel`,"aria-label":`Concatenate project and add residual trace`,children:[(0,T.jsxs)(`div`,{className:`multi-head-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Heads rejoin before the shortcut`}),(0,T.jsx)(`h2`,{children:`Concatenate - project - add`})]}),(0,T.jsx)(`code`,{children:`model width = 2`})]}),(0,T.jsxs)(`div`,{className:`multi-head-join-flow`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`head contexts`}),(0,T.jsx)(`code`,{children:Ie(u.heads.map(e=>e.context))})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`concatenate`}),(0,T.jsx)(`code`,{children:Ie(u.concatenated)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`identity W_o`}),(0,T.jsx)(`code`,{children:Ie(u.projectedAttention)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,T.jsxs)(`div`,{className:i?`multi-head-residual`:`multi-head-residual multi-head-residual--off`,children:[(0,T.jsx)(`small`,{children:i?`${n} residual`:`residual removed`}),(0,T.jsx)(`code`,{children:Ie(i?u.input:[0,0])})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,T.jsxs)(`div`,{className:`multi-head-join-result`,children:[(0,T.jsx)(`small`,{children:`add result`}),(0,T.jsx)(`code`,{children:Ie(u.residualSum)})]})]})]}),(0,T.jsxs)(`section`,{className:`multi-head-norm-panel`,"aria-label":`Layer normalization arithmetic`,children:[(0,T.jsxs)(`div`,{className:`multi-head-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One token - normalize across features`}),(0,T.jsx)(`h2`,{children:o?`Layer normalization`:`Layer normalization bypassed`})]}),(0,T.jsxs)(`div`,{className:`multi-head-output`,children:[(0,T.jsx)(`small`,{children:`block output`}),(0,T.jsx)(`strong`,{children:Ie(u.output)})]})]}),(0,T.jsxs)(`div`,{className:o?`multi-head-norm-flow`:`multi-head-norm-flow multi-head-norm-flow--off`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean`}),(0,T.jsx)(`code`,{children:Fe(u.layerNorm.mean)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`centered`}),(0,T.jsx)(`code`,{children:Ie(u.layerNorm.centered)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`squared deviations`}),(0,T.jsx)(`code`,{children:Ie(u.layerNorm.squaredDeviations)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`variance`}),(0,T.jsx)(`code`,{children:Fe(u.layerNorm.variance)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`sqrt(var + 0.00001)`}),(0,T.jsx)(`code`,{children:Fe(u.layerNorm.denominator)})]}),(0,T.jsxs)(`div`,{className:`multi-head-norm-result`,children:[(0,T.jsx)(`small`,{children:`gamma x normalized + beta`}),(0,T.jsx)(`code`,{children:Ie(u.layerNorm.output)})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`multi-head-controls`,"aria-label":`Multi-head attention controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Inspect one token row`}),(0,T.jsx)(`h2`,{children:`Block controls`}),(0,T.jsx)(`p`,{children:`Both heads stay visible so their different scores and value mixes can be compared on the same causal boundary.`}),(0,T.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:t,children:`Return to single-head weights`}),(0,T.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Open tiny decoder training`}),(0,T.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Multi-head token selection`,children:c.tokenIds.map(e=>(0,T.jsx)(`button`,{"aria-pressed":n===e,type:`button`,onClick:()=>r(e),children:e},e))}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:i,onChange:e=>a(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Add residual token`}),(0,T.jsx)(`small`,{children:`Keep the original embedding on a short route.`})]})]}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:o,onChange:e=>s(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Apply layer normalization`}),(0,T.jsx)(`small`,{children:`Use population variance across this token's features.`})]})]}),(0,T.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected block output`}),(0,T.jsx)(`strong`,{children:Ie(u.output)}),(0,T.jsxs)(`span`,{children:[n,` after both head paths rejoin.`]})]}),(0,T.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,T.jsx)(`span`,{children:`Why keep the heads separate?`}),(0,T.jsx)(`p`,{children:`A softmax row belongs to one head. Concatenation happens only after each head has produced its own context.`})]}),(0,T.jsxs)(`div`,{className:`attention-next-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`A decoder repeats this block across tokens and layers, then adds embeddings, a feed-forward path, logits, loss, and an optimizer.`})]})]})]})}function ze(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function Be(e){return`[${e.map(ze).join(`, `)}]`}function Ve({onShowWeights:e}){let t=(0,l.useMemo)(()=>b(),[]),[n,r]=(0,l.useState)(`blue`),[i,a]=(0,l.useState)(`purple`),[o,s]=(0,l.useState)(!1),c=x(t,n,i),u=t.projections.find(e=>e.id===n),d=t.projections.find(e=>e.id===i),f=o?t.scaledScoreMatrix:t.rawScoreMatrix;return(0,T.jsxs)(`main`,{className:`workspace workspace--attention`,children:[(0,T.jsxs)(`section`,{className:`attention-stage`,"aria-label":`Three-token attention score trace`,children:[(0,T.jsxs)(`div`,{className:`attention-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN12 · attention foundations`}),(0,T.jsx)(`h2`,{children:`Query-key score microscope`}),(0,T.jsx)(`p`,{children:`Give every token three jobs, then open any score cell to see the two multiplications and one addition behind its match strength.`})]}),(0,T.jsx)(`div`,{className:`attention-sequence-chip`,children:`red · blue · purple`})]}),(0,T.jsxs)(`section`,{className:`attention-projection-panel`,"aria-label":`Token projections`,children:[(0,T.jsxs)(`div`,{className:`attention-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One token · three projections`}),(0,T.jsx)(`h2`,{children:`Ask, advertise, carry`})]}),(0,T.jsx)(`p`,{children:`Each row uses the same three learned matrices.`})]}),(0,T.jsxs)(`div`,{className:`attention-projection-table`,children:[(0,T.jsxs)(`div`,{className:`attention-projection-head`,"aria-hidden":`true`,children:[(0,T.jsx)(`span`,{children:`token x`}),(0,T.jsx)(`span`,{children:`query q`}),(0,T.jsx)(`span`,{children:`key k`}),(0,T.jsx)(`span`,{children:`value v`})]}),t.projections.map(e=>(0,T.jsxs)(`div`,{className:`attention-projection-row`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`i`,{className:`attention-token-dot attention-token-dot--${e.id}`}),(0,T.jsx)(`strong`,{children:e.label}),(0,T.jsx)(`code`,{children:Be(e.embedding)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`asks with`}),(0,T.jsx)(`code`,{children:Be(e.query)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`matches with`}),(0,T.jsx)(`code`,{children:Be(e.key)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`carries`}),(0,T.jsx)(`code`,{children:Be(e.value)})]})]},e.id))]})]}),(0,T.jsxs)(`section`,{className:`attention-score-panel`,"aria-label":`Query-key score matrix`,children:[(0,T.jsxs)(`div`,{className:`attention-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Rows ask · columns match`}),(0,T.jsxs)(`h2`,{children:[o?`Scaled`:`Raw`,` query-key scores`]})]}),(0,T.jsx)(`code`,{children:o?`QK^T / sqrt(2)`:`QK^T`})]}),(0,T.jsxs)(`div`,{className:`attention-score-layout`,children:[(0,T.jsxs)(`div`,{className:`attention-score-grid`,role:`grid`,"aria-label":`${o?`Scaled`:`Raw`} attention scores`,children:[(0,T.jsx)(`span`,{className:`attention-grid-corner`,children:`q \\ k`}),t.projections.map(e=>(0,T.jsxs)(`span`,{className:`attention-grid-label`,children:[e.label,` k`]},`key-${e.id}`)),t.projections.flatMap((e,o)=>[(0,T.jsxs)(`span`,{className:`attention-grid-label`,children:[e.label,` q`]},`query-${e.id}`),...t.projections.map((t,s)=>{let c=n===e.id&&i===t.id;return(0,T.jsx)(`button`,{"aria-label":`Select ${e.label} query and ${t.label} key`,"aria-selected":c,className:c?`attention-score-cell attention-score-cell--active`:`attention-score-cell`,role:`gridcell`,type:`button`,onClick:()=>{r(e.id),a(t.id)},children:ze(f[o][s])},`${e.id}-${t.id}`)})])]}),(0,T.jsxs)(`div`,{className:`attention-cell-trace`,"aria-label":`Selected dot-product arithmetic`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Selected cell`}),(0,T.jsxs)(`h3`,{children:[u.label,` asks · `,d.label,` matches`]})]}),(0,T.jsxs)(`div`,{className:`attention-vector-pair`,children:[(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`small`,{children:`query`}),(0,T.jsxs)(`code`,{children:[`q_`,u.id,` = `,Be(u.query)]})]}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`small`,{children:`key`}),(0,T.jsxs)(`code`,{children:[`k_`,d.id,` = `,Be(d.key)]})]})]}),(0,T.jsxs)(`div`,{className:`attention-dot-equation`,children:[(0,T.jsx)(`code`,{children:`${ze(u.query[0])} × ${ze(d.key[0])} + ${ze(u.query[1])} × ${ze(d.key[1])}`}),(0,T.jsxs)(`strong`,{children:[`= `,ze(c.rawScore)]})]}),(0,T.jsxs)(`div`,{className:`attention-products`,children:[`coordinate products `,Be(c.products)]}),o?(0,T.jsxs)(`div`,{className:`attention-scale-equation`,children:[ze(c.rawScore),` / sqrt(2) = `,(0,T.jsx)(`strong`,{children:ze(c.scaledScore)})]}):null]})]})]})]}),(0,T.jsxs)(`aside`,{className:`attention-controls`,"aria-label":`Attention score controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Keep the boundary honest`}),(0,T.jsx)(`h2`,{children:`Score controls`}),(0,T.jsx)(`p`,{children:`A score says how strongly one query matches one key. It does not yet say how much of a value to blend.`}),(0,T.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Apply softmax and causal mask`}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:o,onChange:e=>s(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Scale by sqrt(key dimension)`}),(0,T.jsx)(`small`,{children:`Divide every raw score by sqrt(2).`})]})]}),(0,T.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected score`}),(0,T.jsx)(`strong`,{children:ze(o?c.scaledScore:c.rawScore)}),(0,T.jsxs)(`span`,{children:[u.label,` query → `,d.label,` key`]})]}),(0,T.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,T.jsx)(`span`,{children:`Value waiting downstream`}),(0,T.jsxs)(`code`,{children:[`v_`,d.id,` = `,Be(d.value)]}),(0,T.jsx)(`p`,{children:`This payload does not enter the score calculation.`})]}),(0,T.jsxs)(`div`,{className:`attention-next-note`,children:[(0,T.jsx)(`span`,{children:`What comes next?`}),(0,T.jsx)(`p`,{children:`Open the next view to turn each score row into weights and use those weights to blend the value vectors.`})]})]})]})}function He(){let[e,t]=(0,l.useState)(`scores`);return e===`weights`?(0,T.jsx)(le,{onShowMultiHead:()=>t(`multi-head`),onShowScores:()=>t(`scores`)}):e===`multi-head`?(0,T.jsx)(Re,{onShowDecoder:()=>t(`decoder`),onShowWeights:()=>t(`weights`)}):e===`decoder`?(0,T.jsx)(Ce,{onShowMultiHead:()=>t(`multi-head`)}):(0,T.jsx)(Ve,{onShowWeights:()=>t(`weights`)})}function Ue(e){if(typeof e==`string`)return`string:${e}`;if(typeof e==`number`||typeof e==`bigint`||typeof e==`boolean`||typeof e==`symbol`||e==null)return`${typeof e}:${String(e)}`;try{return`json:${JSON.stringify(e)}`}catch{return`string:${String(e)}`}}function We(e,t){return Ue(e).localeCompare(Ue(t))}var Ge=class extends Error{node;constructor(e){super(`Node not found: ${String(e)}`),this.node=e,this.name=`NodeNotFoundError`}},Ke=class extends Error{edgeId;constructor(e){super(`Edge not found: ${e}`),this.edgeId=e,this.name=`EdgeNotFoundError`}},qe=class extends Error{edgeId;constructor(e){super(`Edge ID already exists: ${e}`),this.edgeId=e,this.name=`DuplicateEdgeIdError`}},Je=class extends Error{constructor(e){super(e),this.name=`MultiDirectedGraphCycleError`}},Ye=class{_allowSelfLoops;_nodes=new Set;_edges=new Map;_outgoing=new Map;_incoming=new Map;_graphProperties={};_nodeProperties=new Map;_edgeProperties=new Map;_nextEdgeId=0;constructor(e={}){this._allowSelfLoops=e.allowSelfLoops??!1}get allowSelfLoops(){return this._allowSelfLoops}get size(){return this._nodes.size}addNode(e,t={}){this._nodes.has(e)||(this._nodes.add(e),this._outgoing.set(e,new Set),this._incoming.set(e,new Set),this._nodeProperties.set(e,{})),Object.assign(this._nodeProperties.get(e),t)}removeNode(e){this.assertNode(e);let t=new Set([...this._outgoing.get(e),...this._incoming.get(e)]);for(let e of t)this.removeEdge(e);this._nodes.delete(e),this._outgoing.delete(e),this._incoming.delete(e),this._nodeProperties.delete(e)}hasNode(e){return this._nodes.has(e)}nodes(){return Array.from(this._nodes)}addEdge(e,t,n=1,r={},i){if(e===t&&!this._allowSelfLoops)throw Error(`Self-loops are not allowed: ${String(e)} -> ${String(t)}`);this.validateWeight(n);let a=i??this.allocateEdgeId();if(this._edges.has(a))throw new qe(a);this.addNode(e),this.addNode(t);let o={id:a,from:e,to:t,weight:n};return this._edges.set(a,o),this._outgoing.get(e).add(a),this._incoming.get(t).add(a),this._edgeProperties.set(a,{...r,weight:n}),a}removeEdge(e){let t=this._edges.get(e);if(t===void 0)throw new Ke(e);this._outgoing.get(t.from).delete(e),this._incoming.get(t.to).delete(e),this._edges.delete(e),this._edgeProperties.delete(e)}hasEdge(e){return this._edges.has(e)}edge(e){let t=this._edges.get(e);if(t===void 0)throw new Ke(e);return t}edges(){return Array.from(this._edges.values())}edgesBetween(e,t){return this.assertNode(e),this.assertNode(t),this.outgoingEdges(e).filter(e=>e.to===t)}outgoingEdges(e){return this.assertNode(e),Array.from(this._outgoing.get(e),e=>this.edge(e))}incomingEdges(e){return this.assertNode(e),Array.from(this._incoming.get(e),e=>this.edge(e))}successors(e){return Array.from(new Set(this.outgoingEdges(e).map(e=>e.to)))}predecessors(e){return Array.from(new Set(this.incomingEdges(e).map(e=>e.from)))}edgeWeight(e){return this.edge(e).weight}graphProperties(){return{...this._graphProperties}}setGraphProperty(e,t){this._graphProperties[e]=t}removeGraphProperty(e){delete this._graphProperties[e]}nodeProperties(e){return this.assertNode(e),{...this._nodeProperties.get(e)??{}}}setNodeProperty(e,t,n){this.assertNode(e),this._nodeProperties.get(e)[t]=n}removeNodeProperty(e,t){this.assertNode(e),delete this._nodeProperties.get(e)[t]}edgeProperties(e){return this.assertEdge(e),{...this._edgeProperties.get(e)??{},weight:this.edgeWeight(e)}}setEdgeProperty(e,t,n){if(this.assertEdge(e),t===`weight`){if(typeof n!=`number`||Number.isNaN(n))throw Error(`Edge property 'weight' must be a number`);this.setEdgeWeight(e,n)}this._edgeProperties.get(e)[t]=n}removeEdgeProperty(e,t){if(this.assertEdge(e),t===`weight`){this.setEdgeWeight(e,1),this._edgeProperties.get(e).weight=1;return}delete this._edgeProperties.get(e)[t]}topologicalSort(){let e=new Map;for(let t of this._nodes)e.set(t,this._incoming.get(t).size);let t=Array.from(this._nodes).filter(t=>e.get(t)===0).sort(We),n=[];for(;t.length>0;){let r=t.shift();n.push(r);for(let n of this.outgoingEdges(r)){let r=e.get(n.to)-1;e.set(n.to,r),r===0&&(t.push(n.to),t.sort(We))}}if(n.length!==this._nodes.size)throw new Je(`Graph contains a cycle: processed ${n.length}/${this._nodes.size} nodes`);return n}hasCycle(){try{return this.topologicalSort(),!1}catch(e){if(e instanceof Je)return!0;throw e}}independentGroups(){let e=new Map;for(let t of this._nodes)e.set(t,this._incoming.get(t).size);let t=Array.from(this._nodes).filter(t=>e.get(t)===0).sort(We),n=[],r=0;for(;t.length>0;){n.push(t),r+=t.length;let i=new Set;for(let n of t)for(let t of this.outgoingEdges(n)){let n=e.get(t.to)-1;e.set(t.to,n),n===0&&i.add(t.to)}t=Array.from(i).sort(We)}if(r!==this._nodes.size)throw new Je(`Graph contains a cycle: processed ${r}/${this._nodes.size} nodes`);return n}toString(){return`MultiDirectedGraph(nodes=${this.size}, edges=${this._edges.size})`}allocateEdgeId(){let e=`e${this._nextEdgeId}`;for(;this._edges.has(e);)this._nextEdgeId+=1,e=`e${this._nextEdgeId}`;return this._nextEdgeId+=1,e}assertNode(e){if(!this._nodes.has(e))throw new Ge(e)}assertEdge(e){if(!this._edges.has(e))throw new Ke(e)}validateWeight(e){if(typeof e!=`number`||Number.isNaN(e))throw Error(`Edge weight must be a number`)}setEdgeWeight(e,t){this.validateWeight(t);let n=this.edge(e);this._edges.set(e,{...n,weight:t})}},Xe=class{graph;constructor(e,t=$e(e)){this.graph=t}input(e,t=e,n={}){return et(this.graph,e,t,n),this}constant(e,t,n={}){return tt(this.graph,e,t,n),this}weightedSum(e,t,n={}){return nt(this.graph,e,t,n),this}activation(e,t,n,r={},i){return rt(this.graph,e,t,n,r,i),this}output(e,t,n=e,r={},i){return it(this.graph,e,t,n,r,i),this}};function Ze(e){return new Xe(e)}function Qe(e){if(e.inputNames.length===0)throw Error(`feed-forward network must have at least one input`);if(e.layers.length===0)throw Error(`feed-forward network must have at least one layer`);let t=Ze(e.name),n=`bias`;t.constant(n,1,{"nn.role":`bias`});let r=e.inputNames.map((e,n)=>{let r=`input_${n}`;return t.input(r,e,{"nn.layer":`input`,"nn.index":n}),r});for(let[i,a]of e.layers.entries()){let o=a.name??`layer_${i}`;at(a,r.length,o);let s=[];for(let e=0;e<a.biases.length;e+=1){let i=`${o}_${e}_sum`,c=`${o}_${e}`;t.weightedSum(i,[...r.map((t,n)=>({from:t,weight:a.weights[n][e],edgeId:`${t}_to_${i}`,properties:{"nn.trainable":!0,"nn.layer":o}})),{from:n,weight:a.biases[e],edgeId:`${n}_to_${i}`,properties:{"nn.trainable":!0,"nn.role":`bias_weight`,"nn.layer":o}}],{"nn.layer":o,"nn.index":e,"nn.role":`weighted_sum`}).activation(c,i,a.activation??`none`,{"nn.layer":o,"nn.index":e,"nn.role":`activation`},`${i}_to_${c}`),s.push(c)}if(i===e.layers.length-1)for(let[e,n]of s.entries()){let r=a.outputNames?.[e]??(s.length===1?`prediction`:`output${e}`);t.output(`${o}_${e}_out`,n,r,{"nn.layer":o,"nn.index":e,"nn.role":`output`},`${n}_to_${o}_${e}_out`)}r=s}return t}function $e(e){let t=new Ye;return t.setGraphProperty(`nn.version`,`0`),e!==void 0&&t.setGraphProperty(`nn.name`,e),t}function et(e,t,n=t,r={}){e.addNode(t,{...r,"nn.op":`input`,"nn.input":n})}function tt(e,t,n,r={}){if(!Number.isFinite(n))throw Error(`constant value must be finite`);e.addNode(t,{...r,"nn.op":`constant`,"nn.value":n})}function nt(e,t,n,r={}){e.addNode(t,{...r,"nn.op":`weighted_sum`});for(let r of n)e.addEdge(r.from,t,r.weight??1,r.properties??{},r.edgeId)}function rt(e,t,n,r,i={},a){return e.addNode(t,{...i,"nn.op":`activation`,"nn.activation":r}),e.addEdge(n,t,1,{},a)}function it(e,t,n,r=t,i={},a){return e.addNode(t,{...i,"nn.op":`output`,"nn.output":r}),e.addEdge(n,t,1,{},a)}function at(e,t,n){if(e.biases.length===0)throw Error(`${n} must have at least one unit`);if(e.weights.length!==t)throw Error(`${n} weight row count must match previous layer width`);if(e.outputNames!==void 0&&e.outputNames.length!==e.biases.length)throw Error(`${n} output name count must match unit count`);for(let[t,r]of e.weights.entries()){if(r.length!==e.biases.length)throw Error(`${n} weight row ${t} width must match bias count`);for(let e of r)if(!Number.isFinite(e))throw Error(`${n} weights must be finite`)}for(let t of e.biases)if(!Number.isFinite(t))throw Error(`${n} biases must be finite`)}var ot=class extends Error{nodeId;edgeId;constructor(e,t,n){super(e),this.nodeId=t,this.edgeId=n,this.name=`NeuralGraphCompileError`}};function st(e){let t=e.topologicalSort(),n=[],r=new Map,i=0,a=()=>`v${i++}`;for(let i of t){let t=e.nodeProperties(i),o=pt(t[`nn.op`],`weighted_sum`);if(o===`input`){let e=a();r.set(i,e),n.push({op:`LOAD_INPUT`,dst:e,inputName:pt(t[`nn.input`],i),sourceNode:i});continue}if(o===`constant`){let e=a();r.set(i,e),n.push({op:`LOAD_CONST`,dst:e,value:mt(t[`nn.value`],i,`nn.value`),sourceNode:i});continue}if(o===`weighted_sum`){let t=[];for(let o of e.incomingEdges(i).sort(ft)){let e=r.get(o.from);if(e===void 0)throw new ot(`Source node has no value: ${o.from}`,o.from,o.id);let i=a(),s=a();n.push({op:`LOAD_EDGE_WEIGHT`,dst:i,edgeId:o.id,sourceEdge:o.id}),n.push({op:`MUL`,dst:s,left:e,right:i,sourceEdge:o.id}),t.push(s)}let o=a();r.set(i,o),n.push({op:t.length===0?`LOAD_CONST`:`ADD`,dst:o,value:t.length===0?0:void 0,inputs:t.length===0?void 0:t,sourceNode:i});continue}if(o===`activation`){let o=dt(e,r,i),s=a();r.set(i,s),n.push({op:`ACTIVATE`,dst:s,input:o,activation:pt(t[`nn.activation`],`relu`),sourceNode:i});continue}if(o===`output`){let a=dt(e,r,i);r.set(i,a),n.push({op:`STORE_OUTPUT`,outputName:pt(t[`nn.output`],i),input:a,sourceNode:i});continue}throw new ot(`Unsupported neural graph op: ${o}`,i)}return{magic:`CANN`,version:0,graph:{nodes:e.nodes(),edges:e.edges().map(e=>({id:e.id,from:e.from,to:e.to,weight:e.weight}))},functions:[{id:`forward`,kind:`forward`,instructions:n}]}}function ct(e){return st(e.graph)}function lt(e,t){return ut(e,t,!0)}function ut(e,t,n){let r=new Map,i=new Map(e.graph.edges.map(e=>[e.id,e.weight])),a={},o=[],s=e.functions.find(e=>e.kind===`forward`);if(s===void 0)throw Error(`Neural bytecode module has no forward function`);for(let[e,c]of s.instructions.entries()){let s=[],l,u,d=e=>{let t=_t(r,e);return s.push({valueId:e,value:t}),t},f=(e,t)=>{r.set(e,t),l={valueId:e,value:t}};switch(c.op){case`LOAD_INPUT`:ht(c),f(c.dst,gt(t,c.inputName));break;case`LOAD_CONST`:ht(c),f(c.dst,c.value??0);break;case`LOAD_EDGE_WEIGHT`:ht(c),f(c.dst,i.get(c.edgeId??``)??1);break;case`MUL`:ht(c),f(c.dst,d(c.left)*d(c.right));break;case`ADD`:ht(c),f(c.dst,(c.inputs??[]).reduce((e,t)=>e+d(t),0));break;case`ACTIVATE`:ht(c),f(c.dst,vt(d(c.input),c.activation??`relu`));break;case`STORE_OUTPUT`:u={outputName:c.outputName??`output`,value:d(c.input)},a[u.outputName]=u.value;break}n&&o.push({index:e,instruction:c,reads:s,write:l,output:u,sourceNode:c.sourceNode,sourceEdge:c.sourceEdge})}return{outputs:a,values:Object.fromEntries(r),instructions:o}}function dt(e,t,n){let r=e.incomingEdges(n).sort(ft);if(r.length!==1)throw new ot(`Expected exactly one input edge for ${n}, got ${r.length}`,n);let i=t.get(r[0].from);if(i===void 0)throw new ot(`Source node has no value: ${r[0].from}`,r[0].from,r[0].id);return i}function ft(e,t){return e.id.localeCompare(t.id)}function pt(e,t){return typeof e==`string`?e:t}function mt(e,t,n){if(typeof e!=`number`||!Number.isFinite(e))throw new ot(`Expected numeric property ${n} on ${t}`,t);return e}function ht(e){if(e.dst===void 0)throw Error(`Instruction ${e.op} is missing dst`)}function gt(e,t){if(t===void 0||!(t in e))throw Error(`Missing input: ${t??`<undefined>`}`);return e[t]}function _t(e,t){if(t===void 0||!e.has(t))throw Error(`Missing value: ${t??`<undefined>`}`);return e.get(t)}function vt(e,t){switch(t){case`relu`:return Math.max(0,e);case`sigmoid`:return 1/(1+Math.exp(-Math.max(-500,Math.min(500,e))));case`tanh`:return Math.tanh(e);case`none`:return e;default:return e}}var yt=class{name=`cpu`;add(e,t){return e.add(t)}subtract(e,t){return e.subtract(t)}scale(e,t){return e.scale(t)}transpose(e){return e.transpose()}dot(e,t){return e.dot(t)}};function bt(e,t,n){if(!Number.isSafeInteger(e)||e<0||e>=t)throw Error(`${n} index ${String(e)} out of bounds for size ${t}.`)}new yt;var xt=class e{data;rows;cols;constructor(e){typeof e==`number`?this.data=[[e]]:Array.isArray(e)&&e.length>0&&typeof e[0]==`number`?this.data=[e]:Array.isArray(e)?this.data=e:this.data=[],this.rows=this.data.length,this.cols=this.rows>0?this.data[0].length:0}static zeros(t,n){return new e(Array.from({length:t},()=>Array(n).fill(0)))}static identity(t){return new e(Array.from({length:t},(e,n)=>Array.from({length:t},(e,t)=>+(n===t))))}static fromDiagonal(t){let n=t.length;return new e(Array.from({length:n},(e,r)=>Array.from({length:n},(e,n)=>r===n?t[r]:0)))}add(t){if(typeof t==`number`)return new e(this.data.map(e=>e.map(e=>e+t)));if(this.rows!==t.rows||this.cols!==t.cols)throw Error(`Add dimension mismatch.`);return new e(this.data.map((e,n)=>e.map((e,r)=>e+t.data[n][r])))}subtract(t){if(typeof t==`number`)return new e(this.data.map(e=>e.map(e=>e-t)));if(this.rows!==t.rows||this.cols!==t.cols)throw Error(`Subtract dimension mismatch.`);return new e(this.data.map((e,n)=>e.map((e,r)=>e-t.data[n][r])))}scale(t){return new e(this.data.map(e=>e.map(e=>e*t)))}transpose(){return this.rows===0?new e([]):new e(this.data[0].map((e,t)=>this.data.map(e=>e[t])))}dot(t){if(this.cols!==t.rows)throw Error(`Dot product inner dimensions strictly mismatch.`);let n=e.zeros(this.rows,t.cols);for(let e=0;e<this.rows;e++)for(let r=0;r<t.cols;r++)for(let i=0;i<this.cols;i++)n.data[e][r]+=this.data[e][i]*t.data[i][r];return n}get(e,t){return bt(e,this.rows,`row`),bt(t,this.cols,`col`),this.data[e][t]}set(t,n,r){bt(t,this.rows,`row`),bt(n,this.cols,`col`);let i=[...this.data[t]];return i.splice(n,1,r),new e(this.data.map((e,n)=>n===t?i:[...e]))}sum(){let e=0;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)e+=this.data[t][n];return e}sumRows(){return new e(this.data.map(e=>[e.reduce((e,t)=>e+t,0)]))}sumCols(){let t=Array(this.cols).fill(0);for(let e=0;e<this.rows;e++)for(let n=0;n<this.cols;n++)t[n]+=this.data[e][n];return new e([t])}mean(){return this.sum()/(this.rows*this.cols)}min(){let e=1/0;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)this.data[t][n]<e&&(e=this.data[t][n]);return e}max(){let e=-1/0;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)this.data[t][n]>e&&(e=this.data[t][n]);return e}argmin(){let e=1/0,t=0,n=0;for(let r=0;r<this.rows;r++)for(let i=0;i<this.cols;i++)this.data[r][i]<e&&(e=this.data[r][i],t=r,n=i);return[t,n]}argmax(){let e=-1/0,t=0,n=0;for(let r=0;r<this.rows;r++)for(let i=0;i<this.cols;i++)this.data[r][i]>e&&(e=this.data[r][i],t=r,n=i);return[t,n]}map(t){return new e(this.data.map(e=>e.map(t)))}sqrt(){return this.map(Math.sqrt)}abs(){return this.map(Math.abs)}pow(e){return this.map(t=>t**+e)}flatten(){let t=[];for(let e=0;e<this.rows;e++)for(let n=0;n<this.cols;n++)t.push(this.data[e][n]);return new e([t])}reshape(t,n){if(t*n!==this.rows*this.cols)throw Error(`Cannot reshape ${this.rows}x${this.cols} to ${t}x${n}.`);let r=this.flatten().data[0],i=[];for(let e=0;e<t;e++)i.push(r.slice(e*n,(e+1)*n));return new e(i)}row(t){if(t<0||t>=this.rows)throw Error(`Row index ${t} out of bounds for ${this.rows} rows.`);return new e([[...this.data[t]]])}col(t){if(t<0||t>=this.cols)throw Error(`Column index ${t} out of bounds for ${this.cols} cols.`);return new e(this.data.map(e=>[e[t]]))}slice(t,n,r,i){if(t<0||n>this.rows||r<0||i>this.cols||t>=n||r>=i)throw Error(`Invalid slice [${t}:${n}, ${r}:${i}] for ${this.rows}x${this.cols} matrix.`);let a=[];for(let e=t;e<n;e++)a.push(this.data[e].slice(r,i));return new e(a)}equals(e){if(this.rows!==e.rows||this.cols!==e.cols)return!1;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)if(this.data[t][n]!==e.data[t][n])return!1;return!0}close(e,t=1e-9){if(this.rows!==e.rows||this.cols!==e.cols)return!1;for(let n=0;n<this.rows;n++)for(let r=0;r<this.cols;r++)if(Math.abs(this.data[n][r]-e.data[n][r])>t)return!1;return!0}},St=class{fromRows(e){return new xt(Ft(e))}toRows(e){return Ft(e.data)}column(e){return new xt(e.map(e=>[e]))}constant(e,t,n=1){return new xt(Array.from({length:t},()=>Array(n).fill(e)))}add(e,t){return e.add(t)}scale(e,t){return e.scale(t)}dot(e,t){return e.dot(t)}map(e,t){return e.map(t)}toColumn(e){if(e.cols!==1)throw Error(`Expected a single-column matrix, got ${e.cols} columns`);return e.data.map(e=>e[0])}},Ct=class{backend=new St;column(e){return this.backend.column(e)}constant(e,t,n=1){return this.backend.constant(e,t,n)}add(e,t){return this.backend.add(e,t)}scale(e,t){return this.backend.scale(e,t)}activate(e,t){return this.backend.map(e,e=>vt(e,t))}toColumn(e){return this.backend.toColumn(e)}};function wt(e){let t=e.functions.find(e=>e.kind===`forward`);if(t===void 0)throw Error(`Neural bytecode module has no forward function`);let n=new Map(e.graph.edges.map(e=>[e.id,e.weight])),r=new Map,i=new Map,a=new Map,o=[];for(let[e,s]of t.instructions.entries())switch(s.op){case`LOAD_INPUT`:{let t=At(s);r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`LOAD_INPUT_MATRIX`,dst:t,inputName:s.inputName,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`LOAD_CONST`:{let t=At(s);r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`LOAD_CONST_MATRIX`,dst:t,value:s.value??0,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`LOAD_EDGE_WEIGHT`:{let e=At(s),t=jt(s);i.set(e,{valueId:e,edgeId:t,weight:n.get(t)??1});break}case`MUL`:{let e=At(s),t=Dt(s,r,i);a.set(e,t);break}case`ADD`:{let t=At(s),n=(s.inputs??[]).map(e=>{let t=a.get(e);if(t===void 0)throw Error(`Cannot lower ADD input ${e} to a matrix term`);return t});r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`WEIGHTED_SUM_MATRIX`,dst:t,terms:n,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`ACTIVATE`:{let t=At(s);Mt(s.input,r),r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`ACTIVATE_MATRIX`,dst:t,input:s.input,activation:s.activation??`relu`,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`STORE_OUTPUT`:Mt(s.input,r),o.push({op:`STORE_OUTPUT_MATRIX`,outputName:s.outputName??`output`,input:s.input,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}return{magic:`CANM`,version:0,sourceBytecodeVersion:e.version,instructions:o}}async function Tt(e,t,n){let r=n??new Ct,i=Ot(t),a=new Map,o=new Set,s={},c=e=>(o.add(e),e);try{for(let n of e.instructions)switch(n.op){case`LOAD_INPUT_MATRIX`:{let e=Nt(n),o=n.inputName??n.sourceNode??e;a.set(e,c(await r.column(kt(t,o,i))));break}case`LOAD_CONST_MATRIX`:{let e=Nt(n);a.set(e,c(await r.constant(n.value??0,i)));break}case`WEIGHTED_SUM_MATRIX`:{let e=Nt(n),t=n.terms??[],o;for(let e of t){let t=c(await r.scale(Pt(a,e.sourceValue),e.weight));o=o===void 0?t:c(await r.add(o,t))}a.set(e,o??c(await r.constant(0,i)));break}case`ACTIVATE_MATRIX`:{let e=Nt(n);a.set(e,c(await r.activate(Pt(a,n.input),n.activation??`relu`)));break}case`STORE_OUTPUT_MATRIX`:{let e=n.outputName??`output`;s[e]=await r.toColumn(Pt(a,n.input));break}}return{outputs:s,values:Object.fromEntries(await Promise.all([...a.entries()].map(async([e,t])=>[e,await r.toColumn(t)])))}}finally{r.dispose!==void 0&&await Promise.all([...o].map(e=>r.dispose(e)))}}function Et(e,t,n=new St){let r=Ot(t),i=new Map,a={};for(let o of e.instructions)switch(o.op){case`LOAD_INPUT_MATRIX`:{let e=Nt(o),a=o.inputName??o.sourceNode??e;i.set(e,n.column(kt(t,a,r)));break}case`LOAD_CONST_MATRIX`:{let e=Nt(o);i.set(e,n.constant(o.value??0,r));break}case`WEIGHTED_SUM_MATRIX`:{let e=Nt(o),t=o.terms??[],a;for(let e of t){let t=n.scale(Pt(i,e.sourceValue),e.weight);a=a===void 0?t:n.add(a,t)}i.set(e,a??n.constant(0,r));break}case`ACTIVATE_MATRIX`:{let e=Nt(o);i.set(e,n.map(Pt(i,o.input),e=>vt(e,o.activation??`relu`)));break}case`STORE_OUTPUT_MATRIX`:{let e=o.outputName??`output`;a[e]=n.toColumn(Pt(i,o.input));break}}return{outputs:a,values:Object.fromEntries([...i.entries()].map(([e,t])=>[e,n.toColumn(t)]))}}function Dt(e,t,n){let r=t.get(e.left??``),i=t.get(e.right??``),a=n.get(e.left??``),o=n.get(e.right??``);if(r!==void 0&&o!==void 0)return{sourceValue:r.valueId,sourceNode:r.sourceNode,edgeId:o.edgeId,weight:o.weight};if(i!==void 0&&a!==void 0)return{sourceValue:i.valueId,sourceNode:i.sourceNode,edgeId:a.edgeId,weight:a.weight};throw Error(`Cannot lower MUL ${e.dst??`<unknown>`} to a weighted matrix term`)}function Ot(e){let t=1;for(let n of Object.values(e))if(Array.isArray(n)){if(n.length===0)throw Error(`Batched inputs must contain at least one value`);if(t!==1&&n.length!==t)throw Error(`All batched inputs must have the same length`);t=n.length}return t}function kt(e,t,n){if(!(t in e))throw Error(`Missing input: ${t}`);let r=e[t];if(Array.isArray(r)){if(r.length!==n)throw Error(`All batched inputs must have the same length`);return[...r]}return Array(n).fill(r)}function At(e){if(e.dst===void 0)throw Error(`Instruction ${e.op} is missing dst`);return e.dst}function jt(e){if(e.edgeId===void 0)throw Error(`LOAD_EDGE_WEIGHT is missing edgeId`);return e.edgeId}function Mt(e,t){if(e===void 0||!t.has(e))throw Error(`Cannot lower missing value: ${e??`<undefined>`}`)}function Nt(e){if(e.dst===void 0)throw Error(`Matrix plan instruction ${e.op} is missing dst`);return e.dst}function Pt(e,t){if(t===void 0||!e.has(t))throw Error(`Missing matrix value: ${t??`<undefined>`}`);return e.get(t)}function Ft(e){return e.map(e=>[...e])}var It={MAP_READ:1,COPY_SRC:4,COPY_DST:8,STORAGE:128},Lt={READ:1},Rt={COMPUTE:4},zt=64,Bt=class e{device;unaryLayout;binaryLayout;scalePipeline;addPipeline;activationPipeline;constructor(e){this.device=e,this.unaryLayout=this.device.createBindGroupLayout({label:`neural-matrix-unary-layout`,entries:[Ht(0,`read-only-storage`),Ht(1,`read-only-storage`),Ht(2,`storage`)]}),this.binaryLayout=this.device.createBindGroupLayout({label:`neural-matrix-binary-layout`,entries:[Ht(0,`read-only-storage`),Ht(1,`read-only-storage`),Ht(2,`storage`)]}),this.scalePipeline=this.createPipeline(`neural-matrix-scale`,Jt,this.unaryLayout),this.addPipeline=this.createPipeline(`neural-matrix-add`,qt,this.binaryLayout),this.activationPipeline=this.createPipeline(`neural-matrix-activation`,Yt,this.unaryLayout)}static async create(t,n={}){let r=await t.requestAdapter(n);if(r===null)throw Error(`WebGPU is available, but no adapter was returned`);return e.createFromAdapter(r)}static async createFromNavigator(t={}){let n=Vt();if(n===void 0)return null;let r=await n.requestAdapter(t);return r===null?null:e.createFromAdapter(r)}static isNavigatorAvailable(){return Vt()!==void 0}static async createFromAdapter(t){let n=await t.requestDevice();try{return new e(n)}catch(e){throw n.destroy?.(),e}}async fromRows(e){let t=e.length,n=e[0]?.length??0,r=new Float32Array(t*n);return e.forEach((e,t)=>{if(e.length!==n)throw Error(`All WebGPU matrix rows must have the same column count`);e.forEach((e,i)=>{r[t*n+i]=e})}),this.upload(r,t,n,`neural-matrix-rows`)}async toRows(e){let t=await this.download(e);return Array.from({length:e.rows},(n,r)=>Array.from(t.slice(r*e.cols,r*e.cols+e.cols)))}column(e){return this.upload(new Float32Array(e),e.length,1,`neural-matrix-column`)}constant(e,t,n=1){return this.upload(new Float32Array(t*n).fill(e),t,n,`neural-matrix-constant`)}add(e,t){Gt(e,t);let n=this.createOutput(e.rows,e.cols,`neural-matrix-add-output`);return this.runBinary(this.addPipeline,e,t,n,`neural-matrix-add-pass`),n}scale(e,t){let n=this.uploadParameter(new Float32Array([t]),`neural-matrix-scale-value`),r=this.createOutput(e.rows,e.cols,`neural-matrix-scale-output`,[n]);return this.runUnary(this.scalePipeline,e,n,r,`neural-matrix-scale-pass`),r}activate(e,t){let n=this.uploadParameter(new Uint32Array([Kt(t)]),`neural-matrix-activation-code`),r=this.createOutput(e.rows,e.cols,`neural-matrix-activation-output`,[n]);return this.runUnary(this.activationPipeline,e,n,r,`neural-matrix-activation-pass`),r}async toColumn(e){if(e.cols!==1)throw Error(`Expected a single-column WebGPU matrix, got ${e.cols} columns`);return Array.from(await this.download(e))}dispose(e){e.buffer.destroy?.(),e.scratch?.forEach(e=>e.destroy?.())}destroy(){this.device.destroy?.()}createPipeline(e,t,n){let r=this.device.createShaderModule({label:`${e}-shader`,code:t}),i=this.device.createPipelineLayout({label:`${e}-pipeline-layout`,bindGroupLayouts:[n]});return this.device.createComputePipeline({label:e,layout:i,compute:{module:r,entryPoint:`main`}})}upload(e,t,n,r){let i=Wt(e.length),a=this.device.createBuffer({label:r,size:i,usage:It.STORAGE|It.COPY_SRC|It.COPY_DST});return e.length>0&&this.device.queue.writeBuffer(a,0,e),{rows:t,cols:n,length:e.length,byteLength:i,buffer:a}}uploadParameter(e,t){let n=this.device.createBuffer({label:t,size:Wt(e.length),usage:It.STORAGE|It.COPY_DST});return this.device.queue.writeBuffer(n,0,e),n}createOutput(e,t,n,r=[]){let i=e*t,a=Wt(i);return{rows:e,cols:t,length:i,byteLength:a,buffer:this.device.createBuffer({label:n,size:a,usage:It.STORAGE|It.COPY_SRC|It.COPY_DST}),scratch:r}}runBinary(e,t,n,r,i){let a=this.device.createBindGroup({label:`${i}-bind-group`,layout:this.binaryLayout,entries:[Ut(0,t.buffer),Ut(1,n.buffer),Ut(2,r.buffer)]});this.dispatch(e,a,r.length,i)}runUnary(e,t,n,r,i){let a=this.device.createBindGroup({label:`${i}-bind-group`,layout:this.unaryLayout,entries:[Ut(0,t.buffer),Ut(1,n),Ut(2,r.buffer)]});this.dispatch(e,a,r.length,i)}dispatch(e,t,n,r){let i=this.device.createCommandEncoder({label:`${r}-encoder`}),a=i.beginComputePass({label:r});a.setPipeline(e),a.setBindGroup(0,t),a.dispatchWorkgroups(Math.max(1,Math.ceil(n/zt))),a.end(),this.device.queue.submit([i.finish()])}async download(e){let t=this.device.createBuffer({label:`neural-matrix-readback`,size:e.byteLength,usage:It.MAP_READ|It.COPY_DST}),n=this.device.createCommandEncoder({label:`neural-matrix-readback-encoder`});n.copyBufferToBuffer(e.buffer,0,t,0,e.byteLength),this.device.queue.submit([n.finish()]),await this.device.queue.onSubmittedWorkDone?.(),await t.mapAsync(Lt.READ,0,e.byteLength);let r=t.getMappedRange(0,e.byteLength),i=(ArrayBuffer.isView(r)?new Uint8Array(r.buffer,r.byteOffset,r.byteLength):new Uint8Array(r)).slice(0,e.length*Float32Array.BYTES_PER_ELEMENT),a=new Float32Array(i.buffer,i.byteOffset,e.length),o=new Float32Array(a);return t.unmap(),t.destroy?.(),o}};function Vt(){return globalThis.navigator?.gpu}function Ht(e,t){return{binding:e,visibility:Rt.COMPUTE,buffer:{type:t}}}function Ut(e,t){return{binding:e,resource:{buffer:t}}}function Wt(e){return Math.max(Float32Array.BYTES_PER_ELEMENT,e*Float32Array.BYTES_PER_ELEMENT)}function Gt(e,t){if(e.rows!==t.rows||e.cols!==t.cols)throw Error(`WebGPU matrix shape mismatch: ${e.rows}x${e.cols} vs ${t.rows}x${t.cols}`)}function Kt(e){switch(e){case`none`:case`linear`:return 0;case`relu`:return 1;case`sigmoid`:return 2;case`tanh`:return 3;default:throw Error(`Unsupported WebGPU activation: ${e}`)}}var qt=`
@group(0) @binding(0) var<storage, read> left_values: array<f32>;
@group(0) @binding(1) var<storage, read> right_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(${zt})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = left_values[index] + right_values[index];
}
`,Jt=`
@group(0) @binding(0) var<storage, read> input_values: array<f32>;
@group(0) @binding(1) var<storage, read> scalar_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(${zt})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = input_values[index] * scalar_values[0];
}
`,Yt=`
@group(0) @binding(0) var<storage, read> input_values: array<f32>;
@group(0) @binding(1) var<storage, read> activation_values: array<u32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

fn apply_activation(value: f32, activation: u32) -> f32 {
  switch activation {
    case 1u: {
      return max(value, 0.0);
    }
    case 2u: {
      return 1.0 / (1.0 + exp(-value));
    }
    case 3u: {
      let doubled = clamp(2.0 * value, -40.0, 40.0);
      let exponent = exp(doubled);
      return (exponent - 1.0) / (exponent + 1.0);
    }
    default: {
      return value;
    }
  }
}

@compute @workgroup_size(${zt})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = apply_activation(input_values[index], activation_values[0]);
}
`,Xt=8,Zt=512,Qt=1e3,$t=0xe8d4a51000,en=1e-5,tn=[{id:`one_row_by_hand`,title:`One row by hand`,summary:`Save one forward row, reverse it, then apply one scalar SGD update.`,initialParameter:.5,learningRate:.1,inputs:[2],targets:[0],gradientBufferBefore:0,divisor:1},{id:`two_row_mean`,title:`The same plan, two-row mean`,summary:`Keep every instruction ID fixed while two row gradients reduce and average.`,initialParameter:1,learningRate:.1,inputs:[2,-1],targets:[1,1],gradientBufferBefore:0,divisor:2},{id:`persistent_buffer`,title:`Continue a persistent buffer`,summary:`Enter with grad_w = 3, add one new row gradient of 2, and keep 5 after SGD.`,initialParameter:.5,learningRate:.1,inputs:[2],targets:[0],gradientBufferBefore:3,divisor:1}];function O(e,t,n=!0){let r=n?$t:Qt;if(!Number.isFinite(e)||Math.abs(e)>r)throw Error(`${t} must be finite and bounded by ${r}`);return e}function nn(e,t){if(typeof e!=`string`||e.length<1||e.length>Zt)throw Error(`${t} must be a bounded string`);return e}function rn(e,t,n){let r=Object.keys(e).sort(),i=[...t].sort();if(r.length!==i.length||r.some((e,t)=>e!==i[t]))throw Error(`${n} must contain exactly ${i.join(`, `)}`)}function an(e,t){if(!Array.isArray(e)||e.length<1||e.length>Xt)throw Error(`${t} must contain 1 to ${Xt} values`);return e.map((e,n)=>{if(typeof e!=`number`)throw Error(`${t}[${n}] must be numeric`);return O(e,`${t}[${n}]`,!1)})}function on(e){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`scenario must be an object`);rn(e,[`id`,`title`,`summary`,`initialParameter`,`learningRate`,`inputs`,`targets`,`gradientBufferBefore`,`divisor`],`scenario`);let t=an(e.inputs,`scenario.inputs`),n=an(e.targets,`scenario.targets`);if(t.length!==n.length)throw Error(`inputs and targets must have the same bounded length`);let r=e.divisor;if(!Number.isInteger(r)||r<1||r>t.length)throw Error(`divisor must be an integer within the batch length`);let i=O(e.initialParameter,`initial parameter`,!1),a=O(e.learningRate,`learning rate`,!1),o=O(e.gradientBufferBefore,`gradient buffer before`,!1);if(a<=0)throw Error(`learning rate must be positive`);return{id:nn(e.id,`scenario.id`),title:nn(e.title,`scenario.title`),summary:nn(e.summary,`scenario.summary`),initialParameter:i,learningRate:a,inputs:t,targets:n,gradientBufferBefore:o,divisor:r}}function sn(e,t,n,r,i={},a=[],o=[],s=[]){return{id:e,op:t,output:n,inputs:r,attributes:i,sourceNodes:a,sourceEdges:o,sourceInstructions:s}}function cn(){return{magic:`CANB`,version:0,instructions:[sn(`b0`,`SEED_LOSS_GRAD`,`d_loss`,[],{value:1},[`loss`]),sn(`b1`,`HALF_SQUARED_ERROR_GRAD`,`d_residual`,[`residual`,`d_loss`],{},[`loss`,`residual`]),sn(`b2`,`PROPAGATE_GRAD`,`d_prediction`,[`d_residual`],{through:`subtract_prediction`},[`residual`,`prediction`]),sn(`b3`,`PARAMETER_LOCAL_GRAD`,`local_d_w`,[`x`,`d_prediction`],{parameter_id:`w`},[`prediction`],[`w`]),sn(`b4`,`ACCUMULATE_GRAD`,`grad_w`,[`grad_w`,`local_d_w`],{parameter_id:`w`,order:`row_ascending`},[],[`w`]),sn(`b5`,`INPUT_GRAD`,`d_x`,[`w`,`d_prediction`],{input_id:`x`},[`x`,`prediction`],[`w`])]}}function ln(){return{magic:`CANO`,version:0,instructions:[sn(`o0`,`READ_GRAD_BUFFER`,`total_d_w`,[`grad_w`],{parameter_id:`w`},[],[`w`]),sn(`o1`,`DIVIDE_GRAD`,`applied_d_w`,[`total_d_w`],{divisor_source:`scenario.divisor`},[],[`w`],[`o0`]),sn(`o2`,`SGD_UPDATE`,`w_next`,[`w`,`applied_d_w`],{learning_rate_source:`scenario.learning_rate`},[],[`w`],[`o1`]),sn(`o3`,`KEEP_GRAD_BUFFER`,`grad_w_after_step`,[`grad_w`],{optimizer_step_zeroes_gradient:!1},[],[`w`],[`o2`])]}}function un(){return{magic:`CANM-TRAIN`,version:0,instructions:[sn(`t0`,`LOAD_SAVED_COLUMN`,`x_col`,[`x`],{saved_value:`x`},[`x`]),sn(`t1`,`LOAD_SAVED_COLUMN`,`residual_col`,[`residual`],{saved_value:`residual`},[`residual`]),sn(`t2`,`LOSS_GRAD_COLUMN`,`d_prediction_col`,[`residual_col`],{loss:`half_squared_error`},[`loss`,`prediction`],[],[`b0`,`b1`,`b2`]),sn(`t3`,`PARAMETER_LOCAL_GRAD_COLUMN`,`local_d_w_col`,[`x_col`,`d_prediction_col`],{parameter_id:`w`},[`prediction`],[`w`],[`b3`]),sn(`t4`,`INPUT_GRAD_COLUMN`,`d_x_col`,[`d_prediction_col`],{input_id:`x`,parameter_id:`w`},[`x`,`prediction`],[`w`],[`b5`]),sn(`t5`,`REDUCE_SUM_GRAD`,`batch_d_w`,[`local_d_w_col`],{order:`row_ascending`,parameter_id:`w`},[],[`w`],[`b4`]),sn(`t6`,`ACCUMULATE_GRAD_BUFFER`,`grad_w`,[`grad_w`,`batch_d_w`],{parameter_id:`w`},[],[`w`],[`b4`]),sn(`t7`,`DIVIDE_GRAD`,`applied_d_w`,[`grad_w`],{divisor_source:`scenario.divisor`},[],[`w`],[`o0`,`o1`]),sn(`t8`,`SGD_UPDATE_SCALAR`,`w_next`,[`w`,`applied_d_w`],{learning_rate_source:`scenario.learning_rate`},[],[`w`],[`o2`]),sn(`t9`,`KEEP_GRAD_BUFFER`,`grad_w_after_step`,[`grad_w`],{optimizer_step_zeroes_gradient:!1},[],[`w`],[`o3`])]}}function dn(e){let t=$e(`nn30_scalar_training`);return et(t,`x`,`x`),nt(t,`prediction_sum`,[{from:`x`,weight:e,edgeId:`w`,properties:{"nn.trainable":!0}}]),rt(t,`prediction`,`prediction_sum`,`none`,{},`sum_to_prediction`),it(t,`out`,`prediction`,`prediction`,{},`prediction_to_out`),t}function fn(e){let t=st(dn(e.initialParameter)),n=wt(t),r=e.inputs.map(e=>O(lt(t,{x:e}).outputs.prediction,`NeuralIR prediction`)),i=Et(n,{x:e.inputs}).outputs.prediction.map(e=>O(e,`MatrixIR prediction`)),a=e.inputs.map(t=>O(e.initialParameter*t,`direct prediction`));return{directOutputs:a,neuralIrOutputs:r,matrixIrOutputs:i,neuralOps:t.functions[0].instructions.map(e=>e.op),matrixOps:n.instructions.map(e=>e.op),maxError:Sn([a,r,i])}}function pn(e,t){let n=t.map((t,n)=>O(t-e.targets[n],`residual ${n}`)),r=n.map((e,t)=>O(.5*e*e,`loss ${t}`));return{x:[...e.inputs],target:[...e.targets],prediction:[...t],residual:n,loss:r}}function mn(e,t,n,r){let i=new Map;i.set(`x`,[...t.x]),i.set(`residual`,[...t.residual]),i.set(`w`,n),i.set(`grad_w`,r);for(let r of e.instructions)switch(r.op){case`SEED_LOSS_GRAD`:i.set(r.output,Array(t.x.length).fill(1));break;case`HALF_SQUARED_ERROR_GRAD`:{let e=_n(i,`residual`),t=_n(i,`d_loss`);i.set(r.output,e.map((e,n)=>O(e*t[n],`d_residual ${n}`)));break}case`PROPAGATE_GRAD`:i.set(r.output,[..._n(i,`d_residual`)]);break;case`PARAMETER_LOCAL_GRAD`:{let e=_n(i,`x`),t=_n(i,`d_prediction`);i.set(r.output,e.map((e,n)=>O(e*t[n],`local_d_w ${n}`)));break}case`ACCUMULATE_GRAD`:{let e=vn(i,`grad_w`);for(let t of _n(i,`local_d_w`))e=O(e+t,`grad_w reduction`);i.set(r.output,e);break}case`INPUT_GRAD`:i.set(r.output,_n(i,`d_prediction`).map((e,t)=>O(n*e,`d_x ${t}`)));break;default:throw Error(`unsupported backward op: ${r.op}`)}let a=_n(i,`local_d_w`),o=bn(a,`backward batch gradient`);return{dLoss:_n(i,`d_loss`),dResidual:_n(i,`d_residual`),dPrediction:_n(i,`d_prediction`),localDW:a,dX:_n(i,`d_x`),gradientBufferBefore:r,batchGradient:o,gradW:vn(i,`grad_w`)}}function hn(e,t,n){let r=new Map([[`w`,t.initialParameter],[`grad_w`,n]]);for(let n of e.instructions)switch(n.op){case`READ_GRAD_BUFFER`:r.set(n.output,yn(r,`grad_w`));break;case`DIVIDE_GRAD`:r.set(n.output,O(yn(r,`total_d_w`)/t.divisor,`applied gradient`));break;case`SGD_UPDATE`:r.set(n.output,O(yn(r,`w`)-t.learningRate*yn(r,`applied_d_w`),`w_next`));break;case`KEEP_GRAD_BUFFER`:r.set(n.output,yn(r,`grad_w`));break;default:throw Error(`unsupported optimizer op: ${n.op}`)}let i=yn(r,`applied_d_w`),a=yn(r,`w_next`);return{parameterBefore:t.initialParameter,appliedGradient:i,parameterDelta:O(a-t.initialParameter,`parameter delta`),parameterAfter:a,gradientBufferAfterStep:yn(r,`grad_w_after_step`)}}function gn(e,t,n){let r=new Map([[`x`,[...n.x]],[`residual`,[...n.residual]],[`w`,t.initialParameter],[`grad_w`,t.gradientBufferBefore]]);for(let n of e.instructions)switch(n.op){case`LOAD_SAVED_COLUMN`:r.set(n.output,[..._n(r,n.inputs[0])]);break;case`LOSS_GRAD_COLUMN`:r.set(n.output,[..._n(r,`residual_col`)]);break;case`PARAMETER_LOCAL_GRAD_COLUMN`:{let e=_n(r,`x_col`),t=_n(r,`d_prediction_col`);r.set(n.output,e.map((e,n)=>O(e*t[n],`matrix local d_w ${n}`)));break}case`INPUT_GRAD_COLUMN`:r.set(n.output,_n(r,`d_prediction_col`).map((e,n)=>O(t.initialParameter*e,`matrix d_x ${n}`)));break;case`REDUCE_SUM_GRAD`:r.set(n.output,bn(_n(r,`local_d_w_col`),`matrix batch gradient`));break;case`ACCUMULATE_GRAD_BUFFER`:r.set(n.output,O(vn(r,`grad_w`)+vn(r,`batch_d_w`),`matrix grad buffer accumulation`));break;case`DIVIDE_GRAD`:r.set(n.output,O(vn(r,`grad_w`)/t.divisor,`matrix applied gradient`));break;case`SGD_UPDATE_SCALAR`:r.set(n.output,O(t.initialParameter-t.learningRate*vn(r,`applied_d_w`),`matrix w_next`));break;case`KEEP_GRAD_BUFFER`:r.set(n.output,vn(r,`grad_w`));break;default:throw Error(`unsupported matrix training op: ${n.op}`)}return{columns:{x:_n(r,`x_col`),residual:_n(r,`residual_col`),dPrediction:_n(r,`d_prediction_col`),localDW:_n(r,`local_d_w_col`),dX:_n(r,`d_x_col`)},gradientBufferBefore:t.gradientBufferBefore,batchGradient:vn(r,`batch_d_w`),gradW:vn(r,`grad_w`),appliedGradient:vn(r,`applied_d_w`),parameterAfter:vn(r,`w_next`),gradientBufferAfterStep:vn(r,`grad_w_after_step`)}}function _n(e,t){let n=e.get(t);if(!Array.isArray(n))throw Error(`missing column: ${t}`);return[...n]}function vn(e,t){let n=e.get(t);if(typeof n!=`number`)throw Error(`missing scalar: ${t}`);return n}function yn(e,t){let n=e.get(t);if(n===void 0)throw Error(`missing scalar: ${t}`);return n}function bn(e,t){let n=0;for(let r of e)n=O(n+r,t);return n}function xn(e,t){let n=0;return t.inputs.forEach((r,i)=>{let a=O(e*r-t.targets[i],`audit residual ${i}`);n=O(n+.5*a*a,`audit loss sum`)}),n}function Sn(e){let t=0;for(let n=0;n<e[0].length;n+=1)for(let r=0;r<e.length;r+=1)for(let i=r+1;i<e.length;i+=1)t=Math.max(t,Math.abs(e[r][n]-e[i][n]));return O(t,`parity error`)}function Cn(e){let t=on(e),n=fn(t),r=pn(t,n.neuralIrOutputs),i=cn(),a=ln(),o=un(),s=mn(i,r,t.initialParameter,t.gradientBufferBefore),c=hn(a,t,s.gradW),l=gn(o,t,r),u=O((xn(t.initialParameter+en,t)-xn(t.initialParameter-en,t))/(2*en),`numerical gradient`),d=O(Math.abs(s.batchGradient-u),`gradient error`),f=Math.max(Math.abs(s.batchGradient-l.batchGradient),Math.abs(s.gradW-l.gradW),Math.abs(c.appliedGradient-l.appliedGradient),Math.abs(c.parameterAfter-l.parameterAfter),Math.abs(c.gradientBufferAfterStep-l.gradientBufferAfterStep));return Tn({scenario:t,forward:n,savedValues:r,backwardIr:i,optimizerIr:a,matrixTrainingIr:o,backward:s,optimizer:c,matrixTraining:l,gradientAudit:{analytical:s.batchGradient,numerical:u,absoluteError:d},maxPathError:O(f,`training path error`)})}function wn(e){let t=tn.find(t=>t.id===e);if(t===void 0)throw Error(`unknown backward/optimizer lowering scenario: ${e}`);return Cn(t)}function Tn(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>Tn(e)),e)}function En(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(10)).toString()}function Dn(e){switch(e.op){case`SEED_LOSS_GRAD`:return`start reverse mode at 1`;case`HALF_SQUARED_ERROR_GRAD`:return`residual x loss seed`;case`PROPAGATE_GRAD`:return`pass through subtraction`;case`PARAMETER_LOCAL_GRAD`:return`x x d_prediction`;case`ACCUMULATE_GRAD`:return`add rows in stable order`;case`INPUT_GRAD`:return`w x d_prediction`;case`READ_GRAD_BUFFER`:return`read persistent grad_w`;case`DIVIDE_GRAD`:return`apply explicit divisor`;case`SGD_UPDATE`:case`SGD_UPDATE_SCALAR`:return`w - rate x gradient`;case`KEEP_GRAD_BUFFER`:return`step does not clear`;case`LOAD_SAVED_COLUMN`:return`load ${e.inputs[0]} rows`;case`LOSS_GRAD_COLUMN`:return`reverse loss as a column`;case`PARAMETER_LOCAL_GRAD_COLUMN`:return`one d_w per row`;case`INPUT_GRAD_COLUMN`:return`one d_x per row`;case`REDUCE_SUM_GRAD`:return`row-ascending reduction`;case`ACCUMULATE_GRAD_BUFFER`:return`add batch sum to persistent grad_w`;default:return e.inputs.join(`, `)}}function On(e){let t=Object.entries(e.attributes);return t.length===0?`none`:t.map(([e,t])=>`${e}=${Array.isArray(t)?`[${t.join(`, `)}]`:String(t)}`).join(`; `)}function kn(e,t){return t===`backward`?e.backwardIr:t===`optimizer`?e.optimizerIr:e.matrixTrainingIr}function An(e,t){let n={b0:e.backward.dLoss,b1:e.backward.dResidual,b2:e.backward.dPrediction,b3:e.backward.localDW,b4:e.backward.gradW,b5:e.backward.dX},r={o0:e.backward.gradW,o1:e.optimizer.appliedGradient,o2:e.optimizer.parameterAfter,o3:e.optimizer.gradientBufferAfterStep},i={t0:e.matrixTraining.columns.x,t1:e.matrixTraining.columns.residual,t2:e.matrixTraining.columns.dPrediction,t3:e.matrixTraining.columns.localDW,t4:e.matrixTraining.columns.dX,t5:e.matrixTraining.batchGradient,t6:e.matrixTraining.gradW,t7:e.matrixTraining.appliedGradient,t8:e.matrixTraining.parameterAfter,t9:e.matrixTraining.gradientBufferAfterStep},a=t.lane===`backward`?n[t.id]:t.lane===`optimizer`?r[t.id]:i[t.id];return typeof a==`number`?En(a):`[${(a??[]).map(En).join(`, `)}]`}function jn({lane:e,selection:t,setSelection:n,stream:r}){let i=e===`matrix`?`forward-lowering-matrix-lane`:`forward-lowering-instruction-lane`,a=e===`matrix`?`Matrix training IR`:e===`backward`?`Backward IR`:`Optimizer IR`;return(0,T.jsx)(`div`,{className:i,children:r.instructions.map(r=>(0,T.jsxs)(`button`,{"aria-label":`Open ${a} ${r.id}, ${r.op}`,"aria-pressed":t.lane===e&&t.id===r.id,onClick:()=>n({lane:e,id:r.id}),type:`button`,children:[(0,T.jsx)(`small`,{children:r.id}),(0,T.jsx)(`strong`,{children:r.op}),(0,T.jsx)(`code`,{children:r.output}),(0,T.jsx)(`span`,{children:Dn(r)})]},r.id))})}function Mn(){let[e,t]=(0,l.useState)(`one_row_by_hand`),[n,r]=(0,l.useState)({lane:`backward`,id:`b3`}),i=(0,l.useMemo)(()=>wn(e),[e]),a=kn(i,n.lane).instructions.find(e=>e.id===n.id);return(0,T.jsxs)(`main`,{className:`workspace workspace--forward-lowering`,children:[(0,T.jsxs)(`section`,{className:`forward-lowering-stage`,children:[(0,T.jsxs)(`header`,{className:`forward-lowering-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN30 - saved values -> backward -> optimizer`}),(0,T.jsx)(`h2`,{children:`Backward and optimizer lowering map`}),(0,T.jsx)(`p`,{children:`Keep one trainable multiplication fixed while reverse mode becomes an executable schedule and SGD remains a separate state transition.`})]}),(0,T.jsxs)(`span`,{className:`forward-lowering-chip`,children:[i.backwardIr.instructions.length,` backward -> `,i.optimizerIr.instructions.length,` optimizer -> `,i.matrixTrainingIr.instructions.length,` matrix ops`]})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-graph`,"aria-label":`Production forward saved values`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`1 - save`}),(0,T.jsx)(`h2`,{children:`The production forward pass leaves evidence`})]}),(0,T.jsxs)(`code`,{children:[`max forward error `,i.forward.maxError.toExponential(1)]})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-edge-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:`NeuralIR`}),(0,T.jsx)(`span`,{children:i.forward.neuralOps.join(` -> `)}),(0,T.jsx)(`strong`,{children:i.forward.neuralIrOutputs.map(En).join(`, `)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:`MatrixIR`}),(0,T.jsx)(`span`,{children:i.forward.matrixOps.join(` -> `)}),(0,T.jsx)(`strong`,{children:i.forward.matrixIrOutputs.map(En).join(`, `)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:`saved contract`}),(0,T.jsx)(`span`,{children:`x, w, prediction, residual`}),(0,T.jsx)(`strong`,{children:`backward may read them`})]})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-parity-table`,role:`table`,"aria-label":`Saved forward row values`,children:[(0,T.jsxs)(`div`,{className:`forward-lowering-parity-head`,role:`row`,children:[(0,T.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`x`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`target`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`prediction`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`residual`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`loss`})]}),i.savedValues.x.map((e,t)=>(0,T.jsxs)(`div`,{role:`row`,children:[(0,T.jsx)(`strong`,{role:`cell`,children:t}),(0,T.jsx)(`code`,{role:`cell`,children:En(e)}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.savedValues.target[t])}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.savedValues.prediction[t])}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.savedValues.residual[t])}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.savedValues.loss[t])})]},t))]})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`Backward instruction stream`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`2 - reverse`}),(0,T.jsx)(`h2`,{children:`Backward produces gradients`})]}),(0,T.jsxs)(`code`,{children:[i.backwardIr.magic,` v`,i.backwardIr.version]})]}),(0,T.jsx)(jn,{lane:`backward`,selection:n,setSelection:r,stream:i.backwardIr})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`Optimizer instruction stream`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`3 - update policy`}),(0,T.jsx)(`h2`,{children:`The optimizer consumes the buffer`})]}),(0,T.jsxs)(`code`,{children:[i.optimizerIr.magic,` v`,i.optimizerIr.version]})]}),(0,T.jsx)(jn,{lane:`optimizer`,selection:n,setSelection:r,stream:i.optimizerIr})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`Matrix training operation stream`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`4 - batch`}),(0,T.jsx)(`h2`,{children:`Columns reduce into shared parameter state`})]}),(0,T.jsxs)(`code`,{children:[i.matrixTrainingIr.magic,` v`,i.matrixTrainingIr.version]})]}),(0,T.jsx)(jn,{lane:`matrix`,selection:n,setSelection:r,stream:i.matrixTrainingIr})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-selection`,"aria-label":`Selected training lowering detail`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`selected translation`}),(0,T.jsx)(`h2`,{children:a?.op})]}),(0,T.jsx)(`code`,{children:n.id})]}),a===void 0?null:(0,T.jsxs)(`div`,{className:`forward-lowering-detail-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`reads -> writes`}),(0,T.jsxs)(`code`,{children:[a.inputs.join(`, `)||`none`,` -> `,a.output]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`observed value`}),(0,T.jsx)(`code`,{children:An(i,n)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`attributes`}),(0,T.jsx)(`code`,{children:On(a)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`graph provenance`}),(0,T.jsx)(`code`,{children:[...a.sourceNodes,...a.sourceEdges].join(`, `)||`none`})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`lowered from`}),(0,T.jsx)(`code`,{children:a.sourceInstructions.join(`, `)||`direct semantic rule`})]})]})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-parity`,"aria-label":`Backward optimizer execution parity`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`5 - prove equivalence`}),(0,T.jsx)(`h2`,{children:`Scalar and matrix training agree`})]}),(0,T.jsxs)(`code`,{children:[`max error `,i.maxPathError.toExponential(1)]})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-parity-table`,role:`table`,"aria-label":`Backward row gradient values`,children:[(0,T.jsxs)(`div`,{className:`forward-lowering-parity-head`,role:`row`,children:[(0,T.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`x`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`target`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`d prediction`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`local d w`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`d x`})]}),i.backward.dPrediction.map((e,t)=>(0,T.jsxs)(`div`,{role:`row`,children:[(0,T.jsx)(`strong`,{role:`cell`,children:t}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.scenario.inputs[t])}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.scenario.targets[t])}),(0,T.jsx)(`code`,{role:`cell`,children:En(e)}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.backward.localDW[t])}),(0,T.jsx)(`code`,{role:`cell`,children:En(i.backward.dX[t])})]},t))]}),(0,T.jsxs)(`div`,{className:`forward-lowering-edge-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:`persistent accumulation`}),(0,T.jsxs)(`span`,{children:[En(i.backward.gradientBufferBefore),` before + `,i.backward.localDW.map(En).join(` + `)]}),(0,T.jsxs)(`strong`,{children:[`grad_w = `,En(i.backward.gradW)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:`explicit divisor`}),(0,T.jsxs)(`span`,{children:[En(i.backward.gradW),` / `,i.scenario.divisor]}),(0,T.jsxs)(`strong`,{children:[`applied = `,En(i.optimizer.appliedGradient)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:`SGD update`}),(0,T.jsxs)(`span`,{children:[En(i.optimizer.parameterBefore),` - `,En(i.scenario.learningRate),` x `,En(i.optimizer.appliedGradient)]}),(0,T.jsxs)(`strong`,{children:[`w_next = `,En(i.optimizer.parameterAfter)]})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`forward-lowering-controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Run shape`}),(0,T.jsx)(`h2`,{children:`Keep the programs fixed`}),(0,T.jsx)(`p`,{children:`Change the number of rows or enter with a nonzero buffer while the programs stay fixed.`}),(0,T.jsx)(`div`,{className:`forward-lowering-scenario-buttons`,children:tn.map(n=>(0,T.jsxs)(`button`,{"aria-label":n.title,"aria-pressed":e===n.id,onClick:()=>t(n.id),type:`button`,children:[(0,T.jsx)(`strong`,{children:n.title}),(0,T.jsx)(`span`,{children:n.summary})]},n.id))}),(0,T.jsxs)(`div`,{className:`forward-lowering-equation`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Paper result`}),(0,T.jsx)(`code`,{children:`loss = 0.5(w x x - target)^2`}),(0,T.jsx)(`code`,{children:`d_w = (prediction - target) x x`}),(0,T.jsx)(`code`,{children:`grad_w = grad_w_before + sum(d_w)`}),(0,T.jsx)(`code`,{children:`w_next = w - rate x (grad_w / divisor)`})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-mental-model`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Gradient audit`}),(0,T.jsx)(`h2`,{children:`Different route, same slope`}),(0,T.jsxs)(`p`,{children:[`Finite difference `,En(i.gradientAudit.numerical),` vs backward `,En(i.gradientAudit.analytical),`; error `,i.gradientAudit.absoluteError.toExponential(1),`.`]})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-mental-model`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Rust boundary`}),(0,T.jsx)(`h2`,{children:`Tensor math, explicit policy`}),(0,T.jsx)(`p`,{children:`Rust may accelerate multiply, add, and ReduceSum. The host still owns saved values, divisor, update timing, and zeroing.`})]})]})]})}var Nn={schema_version:1,id:`dense-backend-parity`,title:`One dense column across CPU, Rust, and accelerator boundaries`,question:`What changes, and what must stay equal, when y = XW + B moves to another backend?`,absolute_tolerance:1e-6,graph:{equation:`y = XW + B`,dtype:`f32`,input_shape:[3,1],weight_shape:[1,1],bias_shape:[3,1],output_shape:[3,1],weight:[2],bias:[1,1,1],matrix_ir_file:`../matrix-ir/00-dense-batch.graph.json`},scenario:{id:`three_row_dense`,inputs:[1,2,3],input_payload_file:`../payloads/00-input-x.f32le.hex`,expected_payload_file:`../payloads/00-expected-output.f32le.hex`,expected:{products:[2,4,6],outputs:[3,5,7]}},lanes:[{id:`scalar_cpu`,title:`Scalar CPU reference`,runtime:`NN00 bytecode interpreter`,precision:`binary64`,availability:`required`,steps:[`load one x`,`multiply by w`,`add b`,`store one y`],residency:[`host:x`,`host:product`,`host:y`],expected_outputs:[3,5,7]},{id:`typescript_matrix_cpu`,title:`TypeScript matrix CPU`,runtime:`NN01 CANM matrix plan`,precision:`binary64`,availability:`required`,steps:[`load x column`,`scale column by w`,`broadcast and add b`,`store y column`],residency:[`host:x[3x1]`,`host:product[3x1]`,`host:y[3x1]`],expected_outputs:[3,5,7]},{id:`rust_matrix_cpu`,title:`Rust matrix CPU core`,runtime:`MatrixIR JSON -> matrix-rust-napi -> matrix-cpu`,precision:`f32`,availability:`required-in-native-test`,steps:[`decode MatrixIR JSON`,`MatMul X by W`,`Add broadcast B`,`download output bytes`],residency:[`host:x bytes`,`rust:x,W,B buffers`,`rust:y buffer`,`host:y bytes`],expected_outputs:[3,5,7]},{id:`webgpu_accelerated`,title:`WebGPU accelerator`,runtime:`NN01 async WebGpuMatrixBackend`,precision:`f32`,availability:`optional-runtime-probe`,steps:[`upload x column`,`scale on device`,`add bias on device`,`download y and value trace`],residency:[`host:x`,`device:x,product,bias,y`,`host:output y`,`host:trace x,bias,y`],expected_outputs:[3,5,7]}]},Pn=/^[a-z][a-z0-9_]{0,63}$/,Fn=512,In=1e6,Ln=[`scalar_cpu`,`typescript_matrix_cpu`,`rust_matrix_cpu`,`webgpu_accelerated`];function Rn(e,t,n){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`${n} must be an object`);let r=Object.keys(e).sort(),i=[...t].sort();if(r.join(`,`)!==i.join(`,`))throw Error(`${n} has unexpected fields`);return e}function zn(e,t){if(typeof e!=`string`||e.length<1||e.length>Fn)throw Error(`${t} must be bounded text`);return e}function Bn(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>In)throw Error(`${t} must be finite and bounded`);return e}function Vn(e,t,n){if(!Array.isArray(e)||e.length!==t)throw Error(`${n} must contain exactly ${t} numbers`);return e.map((e,t)=>Bn(e,`${n}[${t}]`))}function Hn(e,t,n,r){if(!Array.isArray(e)||e.length<t||e.length>n)throw Error(`${r} has an invalid length`);return e.map((e,t)=>zn(e,`${r}[${t}]`))}function Un(e){let t=Rn(e,[`schema_version`,`id`,`title`,`question`,`absolute_tolerance`,`graph`,`scenario`,`lanes`],`backend parity fixture`);if(t.schema_version!==1||t.id!==`dense-backend-parity`)throw Error(`backend parity fixture identity is not canonical`);let n=Bn(t.absolute_tolerance,`absolute tolerance`);if(n!==1e-6)throw Error(`backend parity tolerance is not canonical`);let r=Rn(t.graph,[`equation`,`dtype`,`input_shape`,`weight_shape`,`bias_shape`,`output_shape`,`weight`,`bias`,`matrix_ir_file`],`backend parity graph`);if(r.equation!==`y = XW + B`||r.dtype!==`f32`||r.matrix_ir_file!==`../matrix-ir/00-dense-batch.graph.json`)throw Error(`backend parity graph contract is not canonical`);let i=Vn(r.weight,1,`graph weight`),a=Vn(r.bias,3,`graph bias`),o={input:Vn(r.input_shape,2,`input shape`),weight:Vn(r.weight_shape,2,`weight shape`),bias:Vn(r.bias_shape,2,`bias shape`),output:Vn(r.output_shape,2,`output shape`)};if(i[0]!==2||a.join(`,`)!==`1,1,1`||o.input.join(`,`)!==`3,1`||o.weight.join(`,`)!==`1,1`||o.bias.join(`,`)!==`3,1`||o.output.join(`,`)!==`3,1`)throw Error(`backend parity dense values and shapes are not canonical`);let s=Rn(t.scenario,[`id`,`inputs`,`input_payload_file`,`expected_payload_file`,`expected`],`backend parity scenario`);if(s.id!==`three_row_dense`||s.input_payload_file!==`../payloads/00-input-x.f32le.hex`||s.expected_payload_file!==`../payloads/00-expected-output.f32le.hex`)throw Error(`backend parity scenario contract is not canonical`);let c=Rn(s.expected,[`products`,`outputs`],`scenario expected`),l=Vn(s.inputs,3,`scenario inputs`),u=Vn(c.products,3,`scenario products`),d=Vn(c.outputs,3,`scenario outputs`);if(l.join(`,`)!==`1,2,3`||u.join(`,`)!==`2,4,6`||d.join(`,`)!==`3,5,7`)throw Error(`backend parity scenario values are not canonical`);if(!Array.isArray(t.lanes)||t.lanes.length!==4)throw Error(`backend parity fixture must contain four lanes`);let f=t.lanes.map((e,t)=>{let n=Rn(e,[`id`,`title`,`runtime`,`precision`,`availability`,`steps`,`residency`,`expected_outputs`],`backend lane ${t}`),r=zn(n.id,`backend lane ${t} id`);if(!Pn.test(r)||r!==Ln[t])throw Error(`backend parity lane roster is not canonical`);let i=n.precision;if(i!==`binary64`&&i!==`f32`)throw Error(`backend lane ${t} precision is invalid`);let a=n.availability;if(a!==`required`&&a!==`required-in-native-test`&&a!==`optional-runtime-probe`)throw Error(`backend lane ${t} availability is invalid`);let o=Vn(n.expected_outputs,3,`backend lane ${t} outputs`);if(o.join(`,`)!==d.join(`,`))throw Error(`backend lane ${t} output oracle is dishonest`);return{id:r,title:zn(n.title,`backend lane ${t} title`),runtime:zn(n.runtime,`backend lane ${t} runtime`),precision:i,availability:a,steps:Hn(n.steps,4,4,`backend lane ${t} steps`),residency:Hn(n.residency,3,4,`backend lane ${t} residency`),expectedOutputs:o}});return $n({id:`dense-backend-parity`,title:zn(t.title,`fixture title`),question:zn(t.question,`fixture question`),absoluteTolerance:n,graph:{equation:`y = XW + B`,dtype:`f32`,weight:i[0],bias:a,shapes:o},scenario:{id:`three_row_dense`,inputs:l,products:u,outputs:d},lanes:f})}var Wn=Un(Nn);function Gn(e=Wn){let t=$e(`backend-parity-dense`);return et(t,`x`),tt(t,`bias`,e.graph.bias[0]),nt(t,`dense`,[{from:`x`,weight:e.graph.weight,edgeId:`weight`},{from:`bias`,weight:1,edgeId:`bias`}]),it(t,`output`,`dense`,`y`,{},`dense_to_output`),t}function Kn(){let e=st(Gn());return{bytecode:e,plan:wt(e)}}function qn(e,t){return e.map((e,n)=>{if(!Number.isFinite(e)||Math.abs(e)>In)throw Error(`${t}[${n}] is not finite and bounded`);return e})}function Jn(e,t){return Math.max(...e.map((e,n)=>Math.abs(e-t[n])))}function Yn(e){return e===`rust_matrix_cpu`?`validated-native-fixture`:e===`webgpu_accelerated`?`deterministic-oracle`:`executed-production`}function Xn(){let e=Wn,{bytecode:t,plan:n}=Kn(),r=e.scenario.inputs.map(e=>lt(t,{x:e}).outputs.y),i=Et(n,{x:e.scenario.inputs}).outputs.y??[],a={scalar_cpu:qn(r,`scalar outputs`),typescript_matrix_cpu:qn(i,`matrix outputs`),rust_matrix_cpu:e.scenario.outputs,webgpu_accelerated:e.scenario.outputs.map(e=>Math.fround(e))},o=e.lanes.map(t=>{let n=a[t.id];return{...t,outputs:n,maxAbsoluteError:Jn(n,e.scenario.outputs),evidence:Yn(t.id)}});return $n({fixture:e,products:e.scenario.inputs.map(t=>t*e.graph.weight),scalarInstructionCount:t.functions[0]?.instructions.length??0,matrixOperationCount:n.instructions.length,lanes:o,maxAbsoluteError:Math.max(...o.map(e=>e.maxAbsoluteError))})}async function Zn(e){let{plan:t}=Kn(),n=qn((await Tt(t,{x:Wn.scenario.inputs},e)).outputs.y??[],`accelerated outputs`);if(n.length!==Wn.scenario.outputs.length)throw Error(`accelerated backend returned the wrong output shape`);let r=Jn(n,Wn.scenario.outputs),i=r<=Wn.absoluteTolerance;return{status:`executed`,outputs:n,maxAbsoluteError:r,withinTolerance:i,message:i?`The async backend executed the production matrix plan and matched the oracle.`:`The async backend executed the production matrix plan but missed the tolerance.`}}async function Qn(){if(!Bt.isNavigatorAvailable())return{status:`unavailable`,message:`This browser does not expose WebGPU.`};let e=null;try{return e=await Bt.createFromNavigator({powerPreference:`high-performance`}),e===null?{status:`unavailable`,message:`No WebGPU adapter was available.`}:await Zn(e)}catch(e){return{status:`failed`,message:(e instanceof Error?e.message:`WebGPU execution failed`).slice(0,256)}}finally{e?.destroy()}}function $n(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>$n(e)),e)}function er(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(9)).toString()}function tr(e){switch(e){case`executed-production`:return`executed here`;case`validated-native-fixture`:return`native fixture proof`;case`deterministic-oracle`:return`oracle until probed`}}function nr(){let e=(0,l.useMemo)(()=>Xn(),[]),[t,n]=(0,l.useState)(`rust_matrix_cpu`),[r,i]=(0,l.useState)({status:`not-run`,message:`Run the probe to ask this browser for a real WebGPU adapter.`}),a=e.lanes.find(e=>e.id===t);async function o(){i({status:`running`,message:`Requesting a WebGPU adapter and executing the plan…`}),i(await Qn())}return(0,T.jsxs)(`main`,{className:`workspace workspace--backend-parity`,children:[(0,T.jsxs)(`section`,{className:`backend-parity-stage`,children:[(0,T.jsxs)(`header`,{className:`backend-parity-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN31 · one graph, four execution engines`}),(0,T.jsx)(`h2`,{children:`Backend parity laboratory`}),(0,T.jsx)(`p`,{children:e.fixture.question})]}),(0,T.jsxs)(`span`,{className:`backend-parity-chip`,children:[`max error `,e.maxAbsoluteError.toExponential(1)]})]}),(0,T.jsxs)(`section`,{className:`backend-parity-paper`,"aria-label":`Dense layer hand calculation`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`1 · calculate`}),(0,T.jsx)(`h2`,{children:`Do the middle row on paper`})]}),(0,T.jsx)(`code`,{children:`y = XW + B`})]}),(0,T.jsxs)(`div`,{className:`backend-parity-equation-flow`,children:[(0,T.jsx)(`code`,{children:`x = 2`}),(0,T.jsx)(`span`,{children:`×`}),(0,T.jsx)(`code`,{children:`w = 2`}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsx)(`code`,{children:`4`}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsx)(`code`,{children:`b = 1`}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsx)(`strong`,{children:`5`})]}),(0,T.jsxs)(`div`,{className:`backend-parity-paper-table`,role:`table`,"aria-label":`Hand calculated dense layer rows`,children:[(0,T.jsxs)(`div`,{className:`backend-parity-table-head`,role:`row`,children:[(0,T.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`x`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`x × 2`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`+ 1`})]}),e.fixture.scenario.inputs.map((t,n)=>(0,T.jsxs)(`div`,{role:`row`,children:[(0,T.jsx)(`strong`,{role:`cell`,children:n}),(0,T.jsx)(`code`,{role:`cell`,children:er(t)}),(0,T.jsx)(`code`,{role:`cell`,children:er(e.products[n])}),(0,T.jsx)(`code`,{role:`cell`,children:er(e.fixture.scenario.outputs[n])})]},n))]})]}),(0,T.jsxs)(`section`,{className:`backend-parity-lanes`,"aria-label":`Backend execution lanes`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`2 · schedule`}),(0,T.jsx)(`h2`,{children:`Same graph, different work plans`})]}),(0,T.jsxs)(`code`,{children:[e.scalarInstructionCount,` scalar · `,e.matrixOperationCount,` matrix`]})]}),(0,T.jsx)(`div`,{className:`backend-parity-lane-grid`,children:e.lanes.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Inspect ${e.title}`,"aria-pressed":t===e.id,onClick:()=>n(e.id),type:`button`,children:[(0,T.jsxs)(`small`,{children:[e.precision,` · `,tr(e.evidence)]}),(0,T.jsx)(`strong`,{children:e.title}),(0,T.jsx)(`span`,{children:e.runtime}),(0,T.jsxs)(`code`,{children:[`[`,e.outputs.map(er).join(`, `),`]`]})]},e.id))})]}),(0,T.jsxs)(`section`,{className:`backend-parity-inspector`,"aria-label":`Selected backend detail`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`3 · inspect `,a.precision]}),(0,T.jsx)(`h2`,{children:a.title})]}),(0,T.jsx)(`code`,{children:a.availability})]}),(0,T.jsxs)(`div`,{className:`backend-parity-detail-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`operations`}),(0,T.jsx)(`ol`,{children:a.steps.map(e=>(0,T.jsx)(`li`,{children:e},e))})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`buffer residency`}),(0,T.jsx)(`ol`,{children:a.residency.map(e=>(0,T.jsx)(`li`,{children:(0,T.jsx)(`code`,{children:e})},e))})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`proof`}),(0,T.jsx)(`strong`,{children:tr(a.evidence)}),(0,T.jsxs)(`p`,{children:[`maximum absolute error: `,(0,T.jsx)(`code`,{children:a.maxAbsoluteError.toExponential(1)})]})]})]})]}),(0,T.jsxs)(`section`,{className:`backend-parity-results`,"aria-label":`Backend output parity`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`4 · compare`}),(0,T.jsx)(`h2`,{children:`Every lane meets the same oracle`})]}),(0,T.jsxs)(`code`,{children:[`tolerance `,e.fixture.absoluteTolerance]})]}),(0,T.jsxs)(`div`,{className:`backend-parity-results-table`,role:`table`,"aria-label":`CPU Rust and accelerator outputs`,children:[(0,T.jsxs)(`div`,{className:`backend-parity-table-head`,role:`row`,children:[(0,T.jsx)(`strong`,{role:`columnheader`,children:`lane`}),e.fixture.scenario.inputs.map((e,t)=>(0,T.jsxs)(`strong`,{role:`columnheader`,children:[`row `,t]},t)),(0,T.jsx)(`strong`,{role:`columnheader`,children:`error`})]}),e.lanes.map(e=>(0,T.jsxs)(`div`,{role:`row`,children:[(0,T.jsx)(`strong`,{role:`cell`,children:e.title}),e.outputs.map((e,t)=>(0,T.jsx)(`code`,{role:`cell`,children:er(e)},t)),(0,T.jsx)(`code`,{role:`cell`,children:e.maxAbsoluteError.toExponential(1)})]},e.id))]})]}),(0,T.jsxs)(`section`,{className:`backend-parity-probe`,"aria-label":`WebGPU runtime probe`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`5 · prove the hardware claim`}),(0,T.jsx)(`h2`,{children:`Browser accelerator probe`}),(0,T.jsx)(`p`,{children:r.message}),r.status===`executed`?(0,T.jsxs)(`code`,{children:[`[`,r.outputs.map(er).join(`, `),`] · error `,r.maxAbsoluteError.toExponential(1),` · `,r.withinTolerance?`parity pass`:`parity mismatch`]}):null]}),(0,T.jsxs)(`div`,{className:`backend-parity-probe-status backend-parity-probe-status--${r.status}`,children:[(0,T.jsx)(`strong`,{children:r.status}),(0,T.jsx)(`button`,{disabled:r.status===`running`,onClick:o,type:`button`,children:r.status===`running`?`Running…`:`Run WebGPU probe`})]})]})]}),(0,T.jsxs)(`aside`,{className:`backend-parity-controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Mental model`}),(0,T.jsx)(`h2`,{children:`Meaning above, mechanics below`}),(0,T.jsx)(`p`,{children:`The graph owns the equation. A backend owns scheduling, precision, buffers, and transfers.`}),(0,T.jsxs)(`div`,{className:`backend-parity-rule`,children:[(0,T.jsx)(`code`,{children:`same graph`}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsx)(`code`,{children:`same input`}),(0,T.jsx)(`span`,{children:`→`}),(0,T.jsx)(`strong`,{children:`equal output`})]}),(0,T.jsxs)(`section`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Rust boundary`}),(0,T.jsxs)(`p`,{children:[`MatrixIR JSON and little-endian f32 buffers are shared. The Node-free Rust helper test executes the checked-in bytes through `,(0,T.jsx)(`code`,{children:`matrix-cpu`}),`.`]})]}),(0,T.jsxs)(`section`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Language direction`}),(0,T.jsx)(`p`,{children:`New language ports can replay this oracle natively, then swap in a Rust binding. A stable C ABI remains an explicit future tranche.`})]}),(0,T.jsxs)(`section`,{className:`backend-parity-warning`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Do not confuse`}),(0,T.jsx)(`p`,{children:`Equal answers prove correctness. They do not prove the GPU is faster—or that it ran at all.`})]})]})]})}var rr=[2,1,3,0,4,2],ir=[1,-1,2],ar=[6,-2,10,0],or=.02,sr=1e-6;function cr(e){return e===0?0:e}function lr(e,t){if(e.length===0||t.length===0)throw Error(`Signal and kernel must contain at least one number.`);if(t.length>e.length)throw Error(`The kernel cannot be longer than the signal in valid mode.`);if(![...e,...t].every(Number.isFinite))throw Error(`Signal and kernel values must be finite numbers.`);return Array.from({length:e.length-t.length+1},(n,r)=>{let i=e.slice(r,r+t.length),a=i.map((e,n)=>cr(e*t[n])),o=a.reduce((e,t)=>[...e,e[e.length-1]+t],[0]);return{outputIndex:r,startIndex:r,window:i,products:a,accumulator:o,output:o[o.length-1]}})}function ur(e,t){if(e.length===0||e.length!==t.length)throw Error(`Outputs and targets must have the same non-zero length.`);return e.reduce((e,n,r)=>e+(n-t[r])**2,0)/e.length}function dr(e,t,n){let r=lr(e,t),i=r.map(e=>e.output);if(n.length!==i.length||!n.every(Number.isFinite))throw Error(`Expected ${i.length} finite target values.`);let a=i.map((e,t)=>cr(e-n[t])),o=a.map(e=>cr(2*e/a.length)),s=t.map(()=>0),c=r.map((e,t)=>{let n=o[t],r=e.window.map((e,t)=>{let r=cr(n*e);return s[t]=cr(s[t]+r),r});return{outputIndex:t,window:e.window,outputGradient:n,kernelGradient:r}});return{outputs:i,errors:a,loss:ur(i,n),outputGradients:o,contributions:c,kernelGradient:s}}function fr(e,t,n,r=sr){if(!Number.isFinite(r)||r<=0)throw Error(`Finite-difference epsilon must be positive.`);return t.map((i,a)=>{let o=[...t],s=[...t];o[a]+=r,s[a]-=r;let c=lr(e,o).map(e=>e.output),l=lr(e,s).map(e=>e.output);return(ur(c,n)-ur(l,n))/(2*r)})}function pr(e,t,n,r){if(!Number.isFinite(r)||r<=0)throw Error(`Learning rate must be positive.`);let i=dr(e,t,n),a=t.map((e,t)=>cr(e-r*i.kernelGradient[t])),o=lr(e,a).map(e=>e.output);return{nextKernel:a,nextOutputs:o,nextLoss:ur(o,n)}}function mr(e){let t=e.split(`,`).map(e=>e.trim());if(t.length===0||t.some(e=>e===``))return null;let n=t.map(Number);return n.every(Number.isFinite)?n:null}function hr(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function gr(e){return e.join(`, `)}function _r(){let[e,t]=(0,l.useState)(gr(rr)),[n,r]=(0,l.useState)(gr(ir)),[i,a]=(0,l.useState)(gr(ar)),[o,s]=(0,l.useState)(or),[c,u]=(0,l.useState)(0),[d,f]=(0,l.useState)(0),p=(0,l.useMemo)(()=>mr(e),[e]),m=(0,l.useMemo)(()=>mr(n),[n]),h=(0,l.useMemo)(()=>mr(i),[i]),g=p===null||m===null?`Use comma-separated finite numbers.`:m.length>p.length?`The kernel must fit entirely inside the signal in valid mode.`:null,_=(0,l.useMemo)(()=>g===null?lr(p,m):[],[g,m,p]),v=g===null?h===null?`Use comma-separated finite training targets.`:h.length===_.length?!Number.isFinite(o)||o<=0?`The learning rate must be a positive number.`:null:`Valid mode produces ${_.length} outputs, so enter ${_.length} targets.`:g,y=(0,l.useMemo)(()=>v===null?dr(p,m,h):null,[m,p,h,v]),b=(0,l.useMemo)(()=>v===null?fr(p,m,h):[],[m,p,h,v]),x=(0,l.useMemo)(()=>v===null?pr(p,m,h,o):null,[m,o,p,h,v]),ee=y!==null&&y.kernelGradient.every((e,t)=>Math.abs(e-b[t])<=1e-7);(0,l.useEffect)(()=>{f(e=>Math.min(e,Math.max(_.length-1,0)))},[_.length]);let S=_[d],C=y?.contributions[d],te=S===void 0?-1:S.startIndex+S.window.length;function ne(){t(gr(rr)),r(gr(ir)),a(gr(ar)),s(or),u(0),f(0)}function w(){x!==null&&(r(gr(x.nextKernel)),u(e=>e+1))}return(0,T.jsxs)(`main`,{className:`workspace workspace--convolution`,children:[(0,T.jsxs)(`section`,{className:`convolution-stage`,"aria-label":`Sliding kernel trace`,children:[(0,T.jsxs)(`div`,{className:`convolution-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN05 · spatial networks`}),(0,T.jsx)(`h2`,{children:`Sliding-kernel microscope`}),(0,T.jsx)(`p`,{children:`One small detector reuses the same weights at every position. Select an output to expose the exact window, products, and running sum that made it.`})]}),(0,T.jsx)(`div`,{className:`convolution-mode-chip`,children:`valid · stride 1 · no flip`})]}),S===void 0||p===null||m===null?(0,T.jsx)(`div`,{className:`convolution-error`,role:`alert`,children:g}):(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`section`,{className:`kernel-slide`,"aria-label":`Kernel over signal`,children:[(0,T.jsxs)(`div`,{className:`array-label`,children:[(0,T.jsx)(`span`,{children:`signal`}),(0,T.jsxs)(`code`,{children:[p.length,` values`]})]}),(0,T.jsx)(`div`,{className:`signal-array`,style:{gridTemplateColumns:`repeat(${p.length}, minmax(48px, 1fr))`},children:p.map((e,t)=>(0,T.jsxs)(`div`,{className:t>=S.startIndex&&t<te?`signal-cell signal-cell--active`:`signal-cell`,children:[(0,T.jsxs)(`small`,{children:[`x[`,t,`]`]}),(0,T.jsx)(`strong`,{children:hr(e)})]},`${t}-${e}`))}),(0,T.jsxs)(`div`,{className:`array-label array-label--kernel`,children:[(0,T.jsx)(`span`,{children:`shared kernel`}),(0,T.jsxs)(`code`,{children:[`starts at x[`,S.startIndex,`]`]})]}),(0,T.jsx)(`div`,{className:`kernel-track`,style:{gridTemplateColumns:`repeat(${p.length}, minmax(48px, 1fr))`},children:(0,T.jsx)(`div`,{className:`kernel-window`,style:{gridColumn:`${S.startIndex+1} / span ${m.length}`,gridTemplateColumns:`repeat(${m.length}, minmax(48px, 1fr))`},children:m.map((e,t)=>(0,T.jsxs)(`div`,{className:`kernel-cell`,children:[(0,T.jsxs)(`small`,{children:[`k[`,t,`]`]}),(0,T.jsx)(`strong`,{children:hr(e)})]},`${t}-${e}`))})})]}),(0,T.jsxs)(`section`,{className:`mac-panel`,"aria-label":`Multiply accumulate trace`,children:[(0,T.jsxs)(`div`,{className:`mac-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Output y[`,S.outputIndex,`]`]}),(0,T.jsx)(`h2`,{children:`Multiply, then accumulate`})]}),(0,T.jsx)(`strong`,{className:`mac-result`,children:hr(S.output)})]}),(0,T.jsx)(`div`,{className:`product-grid`,children:S.products.map((e,t)=>(0,T.jsxs)(`div`,{className:`product-card`,children:[(0,T.jsxs)(`small`,{children:[`term `,t+1]}),(0,T.jsxs)(`code`,{children:[hr(S.window[t]),` × `,hr(m[t])]}),(0,T.jsx)(`strong`,{children:hr(e)})]},t))}),(0,T.jsx)(`div`,{className:`accumulator-strip`,"aria-label":`Running accumulator`,children:S.accumulator.map((e,t)=>(0,T.jsxs)(`div`,{className:`accumulator-step`,children:[(0,T.jsx)(`small`,{children:t===0?`start`:`after term ${t}`}),(0,T.jsx)(`strong`,{children:hr(e)})]},t))}),(0,T.jsxs)(`code`,{className:`expanded-equation`,children:[S.window.map((e,t)=>`${hr(e)}×${hr(m[t])}`).join(` + `),` = `,hr(S.output)]})]}),(0,T.jsxs)(`section`,{className:`output-strip`,"aria-label":`Feature map outputs`,children:[(0,T.jsxs)(`div`,{className:`array-label`,children:[(0,T.jsx)(`span`,{children:`feature map`}),(0,T.jsxs)(`code`,{children:[p.length,` - `,m.length,` + 1 = `,_.length]})]}),(0,T.jsx)(`div`,{className:`output-buttons`,children:_.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Select output ${e.outputIndex}`,className:e.outputIndex===d?`output-button output-button--active`:`output-button`,type:`button`,onClick:()=>f(e.outputIndex),children:[(0,T.jsxs)(`small`,{children:[`y[`,e.outputIndex,`]`]}),(0,T.jsx)(`strong`,{children:hr(e.output)})]},e.outputIndex))})]}),(0,T.jsxs)(`section`,{className:`training-panel`,"aria-label":`Shared kernel gradient trace`,children:[(0,T.jsxs)(`div`,{className:`training-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN06 · backward pass`}),(0,T.jsx)(`h2`,{children:`Shared weights collect gradients`}),(0,T.jsx)(`p`,{children:`Every output sends a contribution back to each kernel weight. Columns add because the same weight was reused in every window.`})]}),(0,T.jsxs)(`div`,{className:ee?`gradient-check-badge gradient-check-badge--pass`:`gradient-check-badge`,children:[(0,T.jsx)(`small`,{children:`finite difference`}),(0,T.jsx)(`strong`,{children:ee?`PASS`:`CHECK`})]})]}),y===null||x===null||C===void 0?(0,T.jsx)(`div`,{className:`convolution-error`,role:`alert`,children:v}):(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`div`,{className:`loss-flow`,"aria-label":`Loss before and after proposed step`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`current MSE`}),(0,T.jsx)(`strong`,{children:hr(y.loss)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`− η∇`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`after proposed step`}),(0,T.jsx)(`strong`,{children:hr(x.nextLoss)})]})]}),(0,T.jsxs)(`section`,{className:`selected-gradient-path`,"aria-label":`Selected output gradient path`,children:[(0,T.jsxs)(`div`,{className:`mac-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected path · y[`,d,`]`]}),(0,T.jsx)(`h3`,{children:`One output sends three contributions`})]}),(0,T.jsxs)(`code`,{children:[`dL/dy = 2/`,y.outputs.length,` × `,hr(y.errors[d]),` = `,hr(C.outputGradient)]})]}),(0,T.jsx)(`div`,{className:`product-grid`,children:C.kernelGradient.map((e,t)=>(0,T.jsxs)(`div`,{className:`product-card`,children:[(0,T.jsxs)(`small`,{children:[`toward k[`,t,`]`]}),(0,T.jsxs)(`code`,{children:[hr(C.outputGradient),` × `,hr(C.window[t])]}),(0,T.jsx)(`strong`,{children:hr(e)})]},t))})]}),(0,T.jsx)(`div`,{className:`gradient-table-wrap`,children:(0,T.jsxs)(`table`,{className:`gradient-table`,children:[(0,T.jsx)(`caption`,{children:`Gradient contributions from every reused position`}),(0,T.jsx)(`thead`,{children:(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`col`,children:`weight`}),y.contributions.map(e=>(0,T.jsxs)(`th`,{scope:`col`,children:[`y[`,e.outputIndex,`]`]},e.outputIndex)),(0,T.jsx)(`th`,{scope:`col`,children:`sum`}),(0,T.jsx)(`th`,{scope:`col`,children:`numeric`})]})}),(0,T.jsx)(`tbody`,{children:m.map((e,t)=>(0,T.jsxs)(`tr`,{children:[(0,T.jsxs)(`th`,{scope:`row`,children:[`dL/dk[`,t,`]`]}),y.contributions.map(e=>(0,T.jsx)(`td`,{children:hr(e.kernelGradient[t])},e.outputIndex)),(0,T.jsx)(`td`,{className:`gradient-sum`,children:hr(y.kernelGradient[t])}),(0,T.jsx)(`td`,{children:hr(b[t])})]},t))})]})}),(0,T.jsx)(`div`,{className:`kernel-update-grid`,"aria-label":`Proposed kernel update`,children:m.map((e,t)=>(0,T.jsxs)(`div`,{className:`kernel-update`,children:[(0,T.jsxs)(`small`,{children:[`update k[`,t,`]`]}),(0,T.jsxs)(`code`,{children:[hr(e),` − `,hr(o),` × `,hr(y.kernelGradient[t])]}),(0,T.jsx)(`strong`,{children:hr(x.nextKernel[t])})]},t))})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`convolution-controls`,"aria-label":`Convolution controls`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Change the arithmetic`}),(0,T.jsx)(`h2`,{children:`Signal and detector`}),(0,T.jsx)(`p`,{children:`Use an asymmetric kernel: reversing it should change the outputs.`})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Input signal`}),(0,T.jsx)(`input`,{"aria-label":`Input signal`,value:e,onChange:e=>t(e.target.value)})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Kernel weights`}),(0,T.jsx)(`input`,{"aria-label":`Kernel weights`,value:n,onChange:e=>r(e.target.value)})]}),(0,T.jsxs)(`div`,{className:`convolution-training-controls`,children:[(0,T.jsxs)(`div`,{className:`history__topline`,children:[(0,T.jsx)(`span`,{children:`Train shared weights`}),(0,T.jsxs)(`strong`,{children:[`step `,c]})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Training targets`}),(0,T.jsx)(`input`,{"aria-label":`Training targets`,value:i,onChange:e=>a(e.target.value)})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Learning rate`}),(0,T.jsx)(`input`,{"aria-label":`Convolution learning rate`,min:`0.0001`,step:`0.001`,type:`number`,value:o,onChange:e=>s(Number(e.target.value))})]}),(0,T.jsx)(`button`,{className:`training-step-button`,disabled:x===null,type:`button`,onClick:w,children:`Apply gradient step`})]}),(0,T.jsxs)(`div`,{className:`button-grid`,children:[(0,T.jsx)(`button`,{type:`button`,disabled:d===0,onClick:()=>f(e=>Math.max(e-1,0)),children:`Previous`}),(0,T.jsx)(`button`,{type:`button`,disabled:d>=_.length-1,onClick:()=>f(e=>Math.min(e+1,_.length-1)),children:`Next`}),(0,T.jsx)(`button`,{type:`button`,onClick:ne,children:`Reset fixture`})]}),(0,T.jsxs)(`div`,{className:`convolution-note`,children:[(0,T.jsx)(`span`,{children:`Why “no flip”?`}),(0,T.jsx)(`p`,{children:`Neural libraries usually say convolution while computing cross-correlation. Kernel k[0] multiplies the leftmost value in every window. The NN05 fixture makes this convention testable across languages.`})]}),(0,T.jsxs)(`div`,{className:`convolution-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Images add a second spatial direction; channels and batches add more indexed loops. The same shared-gradient reduction still happens for every trainable filter.`})]})]})]})}var vr=[{id:`small-tanh`,label:`Small tanh`,summary:`Weights and tanh derivatives shrink the chain`,input:1,weights:[.5,.5,.5,.5],activation:`tanh`,target:0},{id:`saturated-tanh`,label:`Saturated tanh`,summary:`Large preactivations make tanh derivatives tiny`,input:1,weights:[3,3,3,3],activation:`tanh`,target:0},{id:`unit-relu`,label:`Unit ReLU`,summary:`Local Jacobians stay at one`,input:1,weights:[1,1,1,1],activation:`relu`,target:0},{id:`large-relu`,label:`Large ReLU`,summary:`Every layer doubles the forward and backward signal`,input:1,weights:[2,2,2,2],activation:`relu`,target:0}];function yr(e){let t=vr.find(t=>t.id===e);if(!t)throw Error(`NN24 unknown gradient scenario: ${e}`);return t}function br(e,t){return t===`tanh`?Math.tanh(e):Math.max(0,e)}function xr(e,t,n){return n===`tanh`?1-t**2:+(e>0)}function Sr(e,t){return .5*(e.weights.reduce((t,n)=>br(n*t,e.activation),t)-e.target)**2}function Cr(e=`small-tanh`,t=1e-6){let n=yr(e);if(!Number.isFinite(t)||t<=0)throw Error(`NN24 finite-difference epsilon must be positive and finite.`);if(!Number.isFinite(n.input)||!Number.isFinite(n.target)||n.weights.length<2||!n.weights.every(Number.isFinite))throw Error(`NN24 scenarios need finite values and at least two weights.`);let r=n.input,i=n.weights.map((e,t)=>{let i=e*r,a=br(i,n.activation),o=xr(i,a,n.activation),s={layer:t+1,input:r,weight:e,preactivation:i,activation:a,activationDerivative:o,localJacobian:e*o,upstreamGradient:0,preactivationGradient:0,weightGradient:0,inputGradient:0};return r=a,s}),a=r,o=a-n.target,s=.5*o**2,c=o;for(let e=i.length-1;e>=0;--e){let t=i[e],n=c*t.activationDerivative;t.upstreamGradient=c,t.preactivationGradient=n,t.weightGradient=n*t.input,t.inputGradient=n*t.weight,c=t.inputGradient}let l=i.reduce((e,t)=>e*t.localJacobian,1),u=(Sr(n,n.input+t)-Sr(n,n.input-t))/(2*t),d=Math.abs(l),f=d<.1?`vanishing`:d>10?`exploding`:`stable`;return{scenario:{...n,weights:[...n.weights]},output:a,outputError:o,loss:s,chainJacobian:l,inputGradient:c,finiteDifferenceInputGradient:u,finiteDifferenceError:Math.abs(c-u),classification:f,layers:i}}function k(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function wr(){let[e,t]=(0,l.useState)(`small-tanh`),[n,r]=(0,l.useState)(3),i=(0,l.useMemo)(()=>Cr(e),[e]),a=(0,l.useMemo)(()=>vr.map(e=>Cr(e.id)),[]),o=i.layers[n],s=Math.max(...a.map(e=>Math.log10(1+Math.abs(e.inputGradient))),1e-12);return(0,T.jsxs)(`main`,{className:`workspace workspace--gradient-flow`,children:[(0,T.jsxs)(`section`,{className:`gradient-flow-stage`,"aria-label":`Vanishing and exploding gradient explorer`,children:[(0,T.jsxs)(`div`,{className:`gradient-flow-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN24 / reverse one scalar chain`}),(0,T.jsx)(`h2`,{children:`Vanishing and exploding gradients`}),(0,T.jsx)(`p`,{children:`Multiply four local Jacobians and watch one loss gradient travel from the output back to the input.`})]}),(0,T.jsx)(`div`,{className:`gradient-flow-chip gradient-flow-chip--${i.classification}`,children:i.classification})]}),(0,T.jsxs)(`section`,{className:`gradient-forward-panel`,"aria-label":`Gradient flow forward pass`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Forward / save every value`}),(0,T.jsx)(`h2`,{children:`Input to loss`})]}),(0,T.jsxs)(`span`,{children:[`half squared error target `,k(i.scenario.target)]})]}),(0,T.jsxs)(`div`,{className:`gradient-forward-lane`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:`input`}),(0,T.jsx)(`strong`,{children:k(i.scenario.input)})]}),i.layers.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`span`,{children:[`layer `,e.layer]}),(0,T.jsxs)(`code`,{children:[k(e.input),` x `,k(e.weight)]}),(0,T.jsxs)(`strong`,{children:[i.scenario.activation,` = `,k(e.activation)]})]},e.layer)),(0,T.jsxs)(`div`,{className:`gradient-loss-node`,children:[(0,T.jsx)(`span`,{children:`loss`}),(0,T.jsx)(`strong`,{children:k(i.loss)})]})]})]}),(0,T.jsxs)(`section`,{className:`gradient-backward-panel`,"aria-label":`Gradient flow backward pass`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Backward / multiply local slopes`}),(0,T.jsx)(`h2`,{children:`Loss to input`})]}),(0,T.jsxs)(`span`,{children:[`start dL/da4 = `,k(i.outputError)]})]}),(0,T.jsxs)(`div`,{className:`gradient-backward-lane`,children:[[...i.layers].reverse().map(e=>(0,T.jsxs)(`button`,{"aria-pressed":n===e.layer-1,type:`button`,onClick:()=>r(e.layer-1),children:[(0,T.jsxs)(`span`,{children:[`layer `,e.layer]}),(0,T.jsxs)(`small`,{children:[`upstream `,k(e.upstreamGradient)]}),(0,T.jsxs)(`strong`,{children:[`local x `,k(e.localJacobian)]}),(0,T.jsxs)(`code`,{children:[`to input `,k(e.inputGradient)]})]},e.layer)),(0,T.jsxs)(`div`,{className:`gradient-input-node`,children:[(0,T.jsx)(`span`,{children:`input gradient`}),(0,T.jsx)(`strong`,{children:k(i.inputGradient)})]})]})]}),(0,T.jsxs)(`section`,{className:`gradient-arithmetic-panel`,"aria-label":`Selected gradient calculation`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Open layer `,o.layer]}),(0,T.jsx)(`h2`,{children:`One chain-rule step`})]}),(0,T.jsxs)(`span`,{children:[`saved input `,k(o.input)]})]}),(0,T.jsxs)(`div`,{className:`gradient-equation-grid`,children:[(0,T.jsxs)(`code`,{children:[k(o.upstreamGradient),` x `,k(o.activationDerivative),` = `,k(o.preactivationGradient)]}),(0,T.jsx)(`span`,{children:`dL/da x da/dz = dL/dz`}),(0,T.jsxs)(`code`,{children:[k(o.preactivationGradient),` x `,k(o.weight),` = `,k(o.inputGradient)]}),(0,T.jsx)(`span`,{children:`dL/dz x dz/dinput`}),(0,T.jsxs)(`code`,{children:[k(o.preactivationGradient),` x `,k(o.input),` = `,k(o.weightGradient)]}),(0,T.jsx)(`span`,{children:`dL/dz x saved input = dL/dw`})]})]}),(0,T.jsxs)(`section`,{className:`gradient-chain-panel`,"aria-label":`Gradient chain product`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Separate the path from the loss`}),(0,T.jsx)(`h2`,{children:`Total local Jacobian product`})]}),(0,T.jsx)(`strong`,{children:k(i.chainJacobian)})]}),(0,T.jsxs)(`div`,{className:`gradient-chain-equation`,children:[i.layers.map(e=>(0,T.jsx)(`code`,{children:k(e.localJacobian)},e.layer)),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsx)(`strong`,{children:k(i.chainJacobian)})]}),(0,T.jsxs)(`p`,{children:[k(i.outputError),` output error x `,k(i.chainJacobian),` chain = `,(0,T.jsx)(`strong`,{children:k(i.inputGradient)}),` input gradient.`]}),(0,T.jsxs)(`div`,{className:`gradient-audit`,children:[(0,T.jsx)(`span`,{children:`central finite difference`}),(0,T.jsx)(`code`,{children:k(i.finiteDifferenceInputGradient)}),(0,T.jsx)(`span`,{children:`absolute error`}),(0,T.jsx)(`code`,{children:k(i.finiteDifferenceError)})]})]}),(0,T.jsxs)(`section`,{className:`gradient-comparison-panel`,"aria-label":`Gradient scenario comparison`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Four mechanisms side by side`}),(0,T.jsx)(`h2`,{children:`Compare input-gradient magnitude`})]}),(0,T.jsx)(`span`,{children:`bar uses log10(1 + |gradient|)`})]}),(0,T.jsx)(`div`,{className:`gradient-comparison-grid`,children:a.map(t=>(0,T.jsxs)(`article`,{className:t.scenario.id===e?`is-selected`:``,children:[(0,T.jsx)(`strong`,{children:t.scenario.label}),(0,T.jsx)(`span`,{children:t.classification}),(0,T.jsx)(`i`,{style:{width:`${Math.log10(1+Math.abs(t.inputGradient))/s*100}%`}}),(0,T.jsxs)(`code`,{children:[`dL/dinput `,k(t.inputGradient)]}),(0,T.jsxs)(`small`,{children:[`chain `,k(t.chainJacobian)]})]},t.scenario.id))})]})]}),(0,T.jsxs)(`aside`,{className:`controls gradient-flow-controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Gradient mechanism`}),(0,T.jsx)(`h2`,{children:`Choose a chain`}),(0,T.jsx)(`p`,{children:`Each scenario keeps four scalar layers and target zero.`}),(0,T.jsx)(`div`,{className:`gradient-scenario-buttons`,children:vr.map(n=>(0,T.jsxs)(`button`,{"aria-pressed":n.id===e,type:`button`,onClick:()=>t(n.id),children:[(0,T.jsx)(`strong`,{children:n.label}),(0,T.jsx)(`span`,{children:n.summary}),(0,T.jsxs)(`code`,{children:[n.weights.join(` x `),` / `,n.activation]})]},n.id))}),(0,T.jsxs)(`div`,{className:`gradient-flow-reading`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`What to notice`}),(0,T.jsx)(`h2`,{children:i.classification===`vanishing`?`Early layers hear a whisper`:i.classification===`exploding`?`Early layers receive a blast`:`The gradient keeps its scale`}),(0,T.jsx)(`p`,{children:`Changing a weight changes both the forward activation and the local factor used on the reverse path.`})]})]})]})}var Tr=[`tiny`,`xavier`,`he`,`large`],Er=[[1,0],[0,1],[-1,0],[0,-1]],Dr=[[[1,-1],[1,1]],[[1,-1],[1,1]],[[1,-1],[1,1]]];function Or(e,t){if(!Number.isInteger(t)||t<1)throw Error(`NN23 fan-in must be a positive integer.`);return e===`tiny`?.1:e===`xavier`?Math.sqrt(1/t):e===`he`?Math.sqrt(2/t):2}function kr(e,t){let n=e.flat();if(n.length===0||!n.every(Number.isFinite))throw Error(`NN23 distributions need at least one finite value.`);let r=n.reduce((e,t)=>e+t,0)/n.length,i=n.reduce((e,t)=>e+(t-r)**2,0)/n.length;return{mean:r,variance:i,standardDeviation:Math.sqrt(i),minimum:Math.min(...n),maximum:Math.max(...n),zeroFraction:n.filter(e=>Math.abs(e)<1e-12).length/n.length,saturatedFraction:t===`tanh`?n.filter(e=>Math.abs(e)>=.95).length/n.length:0}}function Ar(e,t){return t===`tanh`?Math.tanh(e):Math.max(0,e)}function jr(e=`xavier`,t=`tanh`,n=Er,r=Dr){if(!Tr.includes(e))throw Error(`NN23 initializer is not supported.`);if(t!==`tanh`&&t!==`relu`)throw Error(`NN23 activation must be tanh or ReLU.`);if(n.length<2||n[0].length<1)throw Error(`NN23 needs at least two non-empty input rows.`);let i=n[0].length;if(n.some(e=>e.length!==i||!e.every(Number.isFinite)))throw Error(`NN23 inputs must be a finite rectangular matrix.`);if(r.length<1)throw Error(`NN23 needs at least one weight template.`);let a=n.map(e=>[...e]),o=r.map((n,r)=>{let i=a[0].length;if(n.length!==i||n.length===0)throw Error(`NN23 layer ${r+1} template must match fan-in.`);let o=n[0].length;if(o<1||n.some(e=>e.length!==o||!e.every(Number.isFinite)))throw Error(`NN23 layer ${r+1} template must be finite and rectangular.`);let s=Or(e,i),c=n.map(e=>e.map(e=>e*s)),l=a.map(e=>Array.from({length:o},(t,n)=>e.reduce((e,t,r)=>e+t*c[r][n],0))),u=l.map(e=>e.map(e=>Ar(e,t))),d={layer:r+1,fanIn:i,scale:s,weights:c,inputs:a,preactivations:l,activations:u,summary:kr(u,t)};return a=u,d});return{initializer:e,activation:t,inputSummary:kr(n,t),layers:o}}var Mr=[{kind:`tiny`,label:`Tiny`,summary:`fixed scale 0.1`},{kind:`xavier`,label:`Xavier`,summary:`sqrt(1 / fan-in)`},{kind:`he`,label:`He`,summary:`sqrt(2 / fan-in)`},{kind:`large`,label:`Large`,summary:`fixed scale 2`}];function Nr(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function Pr(e,t,n){let r=Math.max(n-t,1e-12);return`${(e-t)/r*100}%`}function Fr(){let[e,t]=(0,l.useState)(`xavier`),[n,r]=(0,l.useState)(`tanh`),[i,a]=(0,l.useState)(0),o=(0,l.useMemo)(()=>jr(e,n),[n,e]),s=(0,l.useMemo)(()=>Tr.map(e=>jr(e,n)),[n]),c=o.layers[i],u=c.inputs[0],d=u.map((e,t)=>e*c.weights[t][0]),f=Math.max(...o.layers.flatMap(e=>e.activations.flat().map(Math.abs)),1),p=Math.max(...s.flatMap(e=>e.layers.map(e=>e.summary.standardDeviation)),1e-12);return(0,T.jsxs)(`main`,{className:`workspace workspace--initialization`,children:[(0,T.jsxs)(`section`,{className:`initialization-stage`,"aria-label":`Initialization distribution explorer`,children:[(0,T.jsxs)(`div`,{className:`lab-intro initialization-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN23 / same signs, different scale`}),(0,T.jsx)(`h2`,{children:`Initialization and activation distributions`}),(0,T.jsx)(`p`,{children:`Follow four tiny inputs through three layers and see when signals shrink, spread, saturate, or explode.`})]}),(0,T.jsxs)(`div`,{className:`initialization-chip`,children:[e,` + `,n]})]}),(0,T.jsxs)(`section`,{className:`initialization-flow`,"aria-label":`Layer activation distributions`,children:[(0,T.jsxs)(`div`,{className:`distribution-card distribution-card--input`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Input batch`}),(0,T.jsx)(`strong`,{children:`4 rows x 2 values`}),(0,T.jsxs)(`code`,{children:[`std `,Nr(o.inputSummary.standardDeviation)]})]}),o.layers.map((e,t)=>{let n=e.activations.flat();return(0,T.jsxs)(`button`,{"aria-pressed":i===t,className:`distribution-card`,type:`button`,onClick:()=>a(t),children:[(0,T.jsxs)(`span`,{className:`eyebrow`,children:[`Layer `,e.layer]}),(0,T.jsxs)(`strong`,{children:[`std `,Nr(e.summary.standardDeviation)]}),(0,T.jsx)(`span`,{className:`distribution-dot-plot`,"aria-hidden":`true`,children:n.map((e,t)=>(0,T.jsx)(`i`,{style:{left:Pr(e,-f,f)}},t))}),(0,T.jsxs)(`span`,{children:[Nr(e.summary.minimum),` to `,Nr(e.summary.maximum)]})]},e.layer)})]}),(0,T.jsxs)(`section`,{className:`distribution-summary-panel`,"aria-label":`Selected activation distribution`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`All eight activations`}),(0,T.jsxs)(`h2`,{children:[`Layer `,c.layer,` distribution`]})]}),(0,T.jsxs)(`span`,{children:[`scale `,Nr(c.scale)]})]}),(0,T.jsxs)(`div`,{className:`distribution-stat-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:`mean`}),(0,T.jsx)(`strong`,{children:Nr(c.summary.mean)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:`variance`}),(0,T.jsx)(`strong`,{children:Nr(c.summary.variance)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:`standard deviation`}),(0,T.jsx)(`strong`,{children:Nr(c.summary.standardDeviation)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:n===`tanh`?`saturated`:`exact zeros`}),(0,T.jsxs)(`strong`,{children:[Nr((n===`tanh`?c.summary.saturatedFraction:c.summary.zeroFraction)*100,3),`%`]})]})]}),(0,T.jsx)(`div`,{className:`activation-value-grid`,children:c.activations.flat().map((e,t)=>(0,T.jsx)(`code`,{children:Nr(e)},t))})]}),(0,T.jsxs)(`section`,{className:`initialization-arithmetic`,"aria-label":`Selected layer hand calculation`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Sample 0 / neuron 0`}),(0,T.jsx)(`h2`,{children:`Open one activation`})]}),(0,T.jsx)(`span`,{children:`no bias in this controlled experiment`})]}),(0,T.jsxs)(`div`,{className:`initialization-equation`,children:[d.map((e,t)=>(0,T.jsxs)(`code`,{children:[Nr(u[t]),` x `,Nr(c.weights[t][0]),` = `,Nr(e)]},t)),(0,T.jsxs)(`strong`,{children:[`sum = `,Nr(c.preactivations[0][0])]}),(0,T.jsxs)(`strong`,{children:[n,` = `,Nr(c.activations[0][0])]})]})]}),(0,T.jsxs)(`section`,{className:`initializer-comparison`,"aria-label":`Initializer comparison`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same inputs / same signs / same activation`}),(0,T.jsx)(`h2`,{children:`Compare signal spread`})]}),(0,T.jsx)(`span`,{children:`bar length = layer standard deviation`})]}),(0,T.jsx)(`div`,{className:`initializer-comparison-grid`,children:s.map(t=>(0,T.jsxs)(`article`,{className:t.initializer===e?`is-selected`:``,children:[(0,T.jsx)(`strong`,{children:t.initializer}),t.layers.map(e=>(0,T.jsxs)(`div`,{className:`spread-row`,children:[(0,T.jsxs)(`span`,{children:[`L`,e.layer]}),(0,T.jsx)(`i`,{style:{width:`${e.summary.standardDeviation/p*100}%`}}),(0,T.jsx)(`code`,{children:Nr(e.summary.standardDeviation)})]},e.layer))]},t.initializer))})]})]}),(0,T.jsxs)(`aside`,{className:`controls initialization-controls`,children:[(0,T.jsxs)(`section`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Weight scale`}),(0,T.jsx)(`h2`,{children:`Choose an initializer`}),(0,T.jsx)(`p`,{children:`The sign template stays fixed so only the scaling rule changes.`}),(0,T.jsx)(`div`,{className:`initializer-buttons`,children:Mr.map(n=>(0,T.jsxs)(`button`,{"aria-pressed":e===n.kind,type:`button`,onClick:()=>t(n.kind),children:[(0,T.jsx)(`span`,{children:n.label}),(0,T.jsx)(`small`,{children:n.summary})]},n.kind))})]}),(0,T.jsxs)(`section`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Nonlinearity`}),(0,T.jsx)(`h2`,{children:`Switch the activation`}),(0,T.jsx)(`div`,{className:`activation-choice-grid`,children:[`tanh`,`relu`].map(e=>(0,T.jsx)(`button`,{"aria-pressed":n===e,type:`button`,onClick:()=>r(e),children:e===`tanh`?`tanh`:`ReLU`},e))})]}),(0,T.jsxs)(`section`,{className:`initialization-reading`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`What to notice`}),(0,T.jsx)(`h2`,{children:e===`tiny`?`Signal is fading`:e===`large`?n===`tanh`?`tanh is pinned near its limits`:`Signal is growing`:`Scale and activation cooperate`}),(0,T.jsx)(`p`,{children:`Real initializers draw random weights. NN23 fixes the signs so every language can reproduce the arithmetic exactly.`})]})]})]})}var Ir=[{id:`plain`,label:`Plain branch`,summary:`The learned branch is the control`},{id:`normalization`,label:`Layer normalization`,summary:`Coordinates share mean and variance`},{id:`dropout`,label:`Inverted dropout`,summary:`A pinned training mask drops and rescales`},{id:`residual`,label:`Identity residual`,summary:`A short skip bypasses the learned branch`}],Lr=[1,1,3,3],Rr=[1,0,0,-1],zr=[1,0,1,0];function Br(e){return Math.abs(e)<1e-12?0:e}function Vr(e,t){return Br(e.reduce((e,n,r)=>e+n*t[r],0))}function Hr(e,t,n){let r=e.reduce((e,t)=>e+t,0)/e.length,i=e.map(e=>Br(e-r)),a=i.reduce((e,t)=>e+t**2,0)/e.length,o=Math.sqrt(a+n);if(o===0)throw Error(`NN25 normalization variance must be positive.`);let s=i.map(e=>Br(e/o));return{mean:r,centered:i,variance:a,standardDeviation:o,normalized:s,upstreamSum:Br(t.reduce((e,t)=>e+t,0)),upstreamDotNormalized:Vr(t,s)}}function Ur(e,t,n,r,i,a){let o=t.map(e=>Br(n*e));return e===`plain`?o:e===`normalization`?Hr(o,[0,0,0,0],a).normalized:e===`dropout`?o.map((e,t)=>Br(e*r[t]/i)):o.map((e,n)=>Br(e+t[n]))}function Wr(e,t,n,r,i,a,o,s,c){let l=Ur(e,t,n,i,a,o),u;if(e===`normalization`){let e=t.length,n=e*c.standardDeviation;u=r.map((t,r)=>Br((e*t-c.upstreamSum-c.normalized[r]*c.upstreamDotNormalized)/n))}else u=e===`dropout`?r.map((e,t)=>Br(e*i[t]/a)):[...r];let d=e===`residual`?[...r]:t.map(()=>0),f=u.map((e,t)=>Br(n*e+d[t])),p=Vr(u,t),m=Vr(r,l),h=t.map((c,l)=>{let u=[...t],d=[...t];return u[l]=u[l]+s,d[l]=d[l]-s,(Vr(r,Ur(e,u,n,i,a,o))-Vr(r,Ur(e,d,n,i,a,o)))/(2*s)}),g=(Vr(r,Ur(e,t,n+s,i,a,o))-Vr(r,Ur(e,t,n-s,i,a,o)))/(2*s);return{id:e,output:l,score:m,branchGradient:u,skipGradient:d,inputGradient:f,weightGradient:p,finiteDifferenceInputGradient:h,finiteDifferenceWeightGradient:g,inputGradientAbsoluteError:f.map((e,t)=>Math.abs(e-h[t])),weightGradientAbsoluteError:Math.abs(p-g)}}function Gr(e=Lr,t=.5,n=Rr,r=zr,i=.5,a=0,o=1e-6){if(e.length!==4||n.length!==4||r.length!==4||!e.every(Number.isFinite)||!n.every(Number.isFinite)||!r.every(e=>e===0||e===1)||!Number.isFinite(t)||!Number.isFinite(i)||i<=0||i>1||!Number.isFinite(a)||a<0||!Number.isFinite(o)||o<=0)throw Error(`NN25 needs four finite coordinates, a binary mask, valid probability, and valid epsilon values.`);let s=e.map(e=>Br(t*e)),c=Hr(s,n,a),l={scaledMask:r.map(e=>Br(e/i)),evaluationOutput:[...s],trainingExpectation:[...s]},u=Ir.map(s=>Wr(s.id,e,t,n,r,i,a,o,c));return{input:[...e],branchWeight:t,upstreamGradient:[...n],dropoutMask:[...r],keepProbability:i,branch:s,normalization:c,dropout:l,routes:u}}function A(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function Kr(e){return`[${e.map(e=>A(e)).join(`, `)}]`}function qr({label:e,values:t,selectedCoordinate:n,tone:r=`blue`}){return(0,T.jsxs)(`div`,{className:`stabilizer-vector stabilizer-vector--${r}`,children:[(0,T.jsx)(`span`,{children:e}),(0,T.jsx)(`div`,{children:t.map((t,r)=>(0,T.jsxs)(`code`,{className:n===r?`is-selected`:``,children:[(0,T.jsx)(`small`,{children:r+1}),A(t)]},`${e}-${r}`))})]})}function Jr(){let[e,t]=(0,l.useState)(`plain`),[n,r]=(0,l.useState)(0),i=(0,l.useMemo)(()=>Gr(),[]),a=i.routes.find(t=>t.id===e),o=Ir.find(t=>t.id===e),s=n,c=Math.max(...a.inputGradientAbsoluteError);return(0,T.jsxs)(`main`,{className:`workspace workspace--stabilizers`,children:[(0,T.jsxs)(`section`,{className:`stabilizer-stage`,"aria-label":`Normalization dropout and residual comparison`,children:[(0,T.jsxs)(`div`,{className:`stabilizer-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN25 / one branch, four routes`}),(0,T.jsx)(`h2`,{children:`Normalization, dropout, and residual paths`}),(0,T.jsx)(`p`,{children:`Hold one learned branch fixed, then watch each training mechanism change its forward values and reverse gradient.`})]}),(0,T.jsx)(`div`,{className:`stabilizer-chip`,children:`4 coordinates`})]}),(0,T.jsxs)(`section`,{className:`stabilizer-common-panel`,"aria-label":`Shared stabilizer branch`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Shared setup`}),(0,T.jsx)(`h2`,{children:`Everything starts from the same branch`})]}),(0,T.jsx)(`span`,{children:`score = upstream · output`})]}),(0,T.jsxs)(`div`,{className:`stabilizer-common-flow`,children:[(0,T.jsx)(qr,{label:`input x`,values:i.input,selectedCoordinate:s}),(0,T.jsxs)(`div`,{className:`stabilizer-flow-arrow`,children:[`× `,A(i.branchWeight)]}),(0,T.jsx)(qr,{label:`learned branch h`,values:i.branch,selectedCoordinate:s,tone:`purple`}),(0,T.jsx)(qr,{label:`upstream dS/doutput`,values:i.upstreamGradient,selectedCoordinate:s,tone:`red`})]})]}),(0,T.jsxs)(`section`,{className:`stabilizer-comparison-panel`,"aria-label":`Training stabilizer route comparison`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same numbers, different jobs`}),(0,T.jsx)(`h2`,{children:`Compare all four routes`})]}),(0,T.jsx)(`span`,{children:`select a route to unpack it`})]}),(0,T.jsx)(`div`,{className:`stabilizer-comparison-grid`,children:i.routes.map(n=>{let r=Ir.find(e=>e.id===n.id);return(0,T.jsxs)(`button`,{"aria-pressed":n.id===e,type:`button`,onClick:()=>t(n.id),children:[(0,T.jsx)(`strong`,{children:r.label}),(0,T.jsx)(`span`,{children:r.summary}),(0,T.jsxs)(`code`,{children:[`output `,Kr(n.output)]}),(0,T.jsxs)(`code`,{children:[`dS/dx `,Kr(n.inputGradient)]}),(0,T.jsxs)(`small`,{children:[`score `,A(n.score)]})]},n.id)})})]}),(0,T.jsxs)(`section`,{className:`stabilizer-forward-panel`,"aria-label":`Selected stabilizer forward trace`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Forward / `,o.label]}),(0,T.jsx)(`h2`,{children:`What changes on this route?`})]}),(0,T.jsxs)(`strong`,{children:[`score `,A(a.score)]})]}),e===`normalization`?(0,T.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,T.jsxs)(`div`,{className:`stabilizer-stat-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean`}),(0,T.jsx)(`strong`,{children:A(i.normalization.mean)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`variance / population`}),(0,T.jsx)(`strong`,{children:A(i.normalization.variance)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`standard deviation`}),(0,T.jsx)(`strong`,{children:A(i.normalization.standardDeviation)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`epsilon / hand fixture`}),(0,T.jsx)(`strong`,{children:`0`})]})]}),(0,T.jsx)(qr,{label:`centered h - mean`,values:i.normalization.centered,selectedCoordinate:s,tone:`purple`}),(0,T.jsx)(qr,{label:`normalized output`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,T.jsx)(`code`,{className:`stabilizer-formula`,children:`normalized[i] = (h[i] - mean) / standard deviation`})]}):e===`dropout`?(0,T.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,T.jsx)(qr,{label:`binary mask`,values:i.dropoutMask,selectedCoordinate:s,tone:`red`}),(0,T.jsx)(qr,{label:`mask / keep probability`,values:i.dropout.scaledMask,selectedCoordinate:s,tone:`purple`}),(0,T.jsx)(qr,{label:`training output`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,T.jsxs)(`div`,{className:`stabilizer-dropout-compare`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`evaluation / dropout off`}),(0,T.jsx)(`code`,{children:Kr(i.dropout.evaluationOutput)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`expectation over training masks`}),(0,T.jsx)(`code`,{children:Kr(i.dropout.trainingExpectation)})]})]}),(0,T.jsxs)(`code`,{className:`stabilizer-formula`,children:[`training output[i] = h[i] × mask[i] / `,A(i.keepProbability)]})]}):e===`residual`?(0,T.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,T.jsx)(qr,{label:`identity skip x`,values:i.input,selectedCoordinate:s}),(0,T.jsx)(`div`,{className:`stabilizer-plus`,children:`+`}),(0,T.jsx)(qr,{label:`learned branch h`,values:i.branch,selectedCoordinate:s,tone:`purple`}),(0,T.jsx)(`div`,{className:`stabilizer-plus`,children:`=`}),(0,T.jsx)(qr,{label:`residual output`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,T.jsx)(`code`,{className:`stabilizer-formula`,children:`output[i] = input[i] + branch[i]`})]}):(0,T.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,T.jsx)(qr,{label:`plain output = h`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,T.jsxs)(`code`,{className:`stabilizer-formula`,children:[`No extra route: output[i] = `,A(i.branchWeight),` × input[i]`]})]})]}),(0,T.jsxs)(`section`,{className:`stabilizer-backward-panel`,"aria-label":`Selected stabilizer backward trace`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Backward / vector-Jacobian product`}),(0,T.jsx)(`h2`,{children:`Where does the score gradient travel?`})]}),(0,T.jsxs)(`span`,{children:[`dS/dweight `,A(a.weightGradient)]})]}),(0,T.jsxs)(`div`,{className:`stabilizer-gradient-flow`,children:[(0,T.jsx)(qr,{label:`upstream`,values:i.upstreamGradient,selectedCoordinate:s,tone:`red`}),(0,T.jsx)(qr,{label:`into learned branch`,values:a.branchGradient,selectedCoordinate:s,tone:`purple`}),e===`residual`?(0,T.jsx)(qr,{label:`through identity skip`,values:a.skipGradient,selectedCoordinate:s}):null,(0,T.jsx)(qr,{label:`total dS/dinput`,values:a.inputGradient,selectedCoordinate:s,tone:`green`})]})]}),(0,T.jsxs)(`section`,{className:`stabilizer-arithmetic-panel`,"aria-label":`Selected stabilizer coordinate calculation`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Open coordinate `,s+1]}),(0,T.jsx)(`h2`,{children:`One reverse calculation`})]}),(0,T.jsxs)(`span`,{children:[`input `,A(i.input[s])]})]}),(0,T.jsxs)(`div`,{className:`stabilizer-equations`,children:[e===`normalization`?(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`code`,{children:[`(4 × `,A(i.upstreamGradient[s]),` - `,A(i.normalization.upstreamSum),` - `,A(i.normalization.normalized[s]),` × `,A(i.normalization.upstreamDotNormalized),`) / (4 × `,A(i.normalization.standardDeviation),`) = `,A(a.branchGradient[s])]}),(0,T.jsx)(`span`,{children:`layer norm couples this coordinate to both vector-wide sums`})]}):e===`dropout`?(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`code`,{children:[A(i.upstreamGradient[s]),` × `,A(i.dropoutMask[s]),` / `,A(i.keepProbability),` = `,A(a.branchGradient[s])]}),(0,T.jsx)(`span`,{children:`a dropped coordinate receives zero branch gradient`})]}):(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`code`,{children:[`dS/dh[`,s+1,`] = `,A(a.branchGradient[s])]}),(0,T.jsx)(`span`,{children:e===`residual`?`the branch and skip both receive the upstream gradient`:`the plain branch passes the upstream gradient unchanged`})]}),(0,T.jsxs)(`code`,{children:[A(i.branchWeight),` × `,A(a.branchGradient[s]),` + `,A(a.skipGradient[s]),` = `,A(a.inputGradient[s])]}),(0,T.jsx)(`span`,{children:`branch contribution + identity-skip contribution`}),(0,T.jsxs)(`code`,{children:[`Σ dS/dh[i] × input[i] = `,A(a.weightGradient)]}),(0,T.jsx)(`span`,{children:`the shared scalar branch-weight gradient`})]})]}),(0,T.jsxs)(`section`,{className:`stabilizer-audit-panel`,"aria-label":`Training stabilizer finite difference audit`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Independent numerical audit`}),(0,T.jsx)(`h2`,{children:`Analytical gradients match score slopes`})]}),(0,T.jsx)(`span`,{children:`epsilon 1e-6`})]}),(0,T.jsxs)(`div`,{className:`stabilizer-audit-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`selected analytical dS/dx`}),(0,T.jsx)(`code`,{children:A(a.inputGradient[s])})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`selected finite difference`}),(0,T.jsx)(`code`,{children:A(a.finiteDifferenceInputGradient[s])})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`maximum input error`}),(0,T.jsx)(`code`,{children:A(c)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`analytical dS/dweight`}),(0,T.jsx)(`code`,{children:A(a.weightGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`weight finite difference`}),(0,T.jsx)(`code`,{children:A(a.finiteDifferenceWeightGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`weight error`}),(0,T.jsx)(`code`,{children:A(a.weightGradientAbsoluteError)})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`controls stabilizer-controls`,"aria-label":`Training stabilizer controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Training mechanism`}),(0,T.jsx)(`h2`,{children:`Choose a route`}),(0,T.jsx)(`p`,{children:`The learned branch, input, and upstream vector stay fixed.`}),(0,T.jsx)(`div`,{className:`stabilizer-route-buttons`,children:Ir.map(n=>(0,T.jsxs)(`button`,{"aria-pressed":n.id===e,type:`button`,onClick:()=>t(n.id),children:[(0,T.jsx)(`strong`,{children:n.label}),(0,T.jsx)(`span`,{children:n.summary})]},n.id))}),(0,T.jsx)(`p`,{className:`eyebrow`,children:`Coordinate microscope`}),(0,T.jsx)(`div`,{className:`stabilizer-coordinate-buttons`,children:i.input.map((e,t)=>(0,T.jsxs)(`button`,{"aria-label":`Open stabilizer coordinate ${t+1}`,"aria-pressed":s===t,type:`button`,onClick:()=>r(t),children:[(0,T.jsx)(`span`,{children:t+1}),(0,T.jsxs)(`code`,{children:[`x = `,A(e)]})]},t))}),(0,T.jsxs)(`div`,{className:`stabilizer-reading`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Different jobs`}),(0,T.jsx)(`h2`,{children:e===`normalization`?`Coordinates share context`:e===`dropout`?`Training samples a subnetwork`:e===`residual`?`The skip keeps a short route`:`The control exposes the branch`}),(0,T.jsx)(`p`,{children:`These mechanisms can coexist, but they are not interchangeable fixes for depth.`})]})]})]})}function Yr(){let[e,t]=(0,l.useState)(`initialization`);return(0,T.jsxs)(`div`,{className:`deep-training-workbench`,children:[(0,T.jsxs)(`nav`,{className:`deep-training-switch`,"aria-label":`Deep training learning lab`,children:[(0,T.jsx)(`button`,{"aria-pressed":e===`initialization`,type:`button`,onClick:()=>t(`initialization`),children:`Initialization`}),(0,T.jsx)(`button`,{"aria-pressed":e===`gradient-flow`,type:`button`,onClick:()=>t(`gradient-flow`),children:`Gradient flow`}),(0,T.jsx)(`button`,{"aria-pressed":e===`stabilizers`,type:`button`,onClick:()=>t(`stabilizers`),children:`Stabilizers`})]}),e===`initialization`?(0,T.jsx)(Fr,{}):e===`gradient-flow`?(0,T.jsx)(wr,{}):(0,T.jsx)(Jr,{})]})}var Xr=4,Zr=12,Qr=1e6,$r=/^[a-z][a-z0-9_]{0,31}$/,ei=[{id:`multiply_add_square`,title:`Complete graph`,summary:`Multiply, add, and square with every saved value visible.`,expression:`loss = (x × w + b)²`,inputs:[{id:`x`,value:2,requiresGradient:!0},{id:`w`,value:3,requiresGradient:!0},{id:`b`,value:1,requiresGradient:!0}],steps:[{id:`m`,operation:`multiply`,inputs:[`x`,`w`]},{id:`z`,operation:`add`,inputs:[`m`,`b`]},{id:`loss`,operation:`square`,inputs:[`z`]}],output:`loss`,mutationsAfterForward:{}},{id:`negative_branch`,title:`Runtime branch`,summary:`A negative input records negate, not the unexecuted identity path.`,expression:`loss = abs(x)², x < 0`,inputs:[{id:`x`,value:-2,requiresGradient:!0}],steps:[{id:`abs_x`,operation:`branch_nonnegative`,inputs:[`x`]},{id:`loss`,operation:`square`,inputs:[`abs_x`]}],output:`loss`,mutationsAfterForward:{}},{id:`saved_snapshot`,title:`Mutation snapshot`,summary:`Live w becomes 100; backward still reads saved forward w = 3.`,expression:`product = x × w; then live w ← 100`,inputs:[{id:`x`,value:2,requiresGradient:!0},{id:`w`,value:3,requiresGradient:!0}],steps:[{id:`product`,operation:`multiply`,inputs:[`x`,`w`]}],output:`product`,mutationsAfterForward:{w:100}}];function ti(e,t){if(!Number.isFinite(e))throw Error(`${t} must remain finite`);return e}function ni(e,t){if(typeof e!=`string`||!$r.test(e))throw Error(`${t} must be a bounded identifier`)}function ri(e,t,n=0){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>Qr+n)throw Error(`${t} must be finite and bounded`)}function ii(e){let t=Object.create(null);Object.entries(e.mutationsAfterForward).forEach(([e,n])=>{t[e]=n});let n=e.inputs.map(e=>Object.freeze({...e})),r=e.steps.map(e=>Object.freeze({...e,inputs:Object.freeze([...e.inputs])}));return Object.freeze({id:e.id,title:e.title,summary:e.summary,expression:e.expression,inputs:Object.freeze(n),steps:Object.freeze(r),output:e.output,mutationsAfterForward:Object.freeze(t)})}function ai(e){return e===`multiply`||e===`add`?2:1}function oi(e){if(typeof e!=`object`||!e||!Array.isArray(e.inputs)||!Array.isArray(e.steps)||typeof e.mutationsAfterForward!=`object`||e.mutationsAfterForward===null||Array.isArray(e.mutationsAfterForward))throw Error(`autograd scenario must contain bounded arrays and mutation object`);if(e.inputs.length<1||e.inputs.length>Xr||e.steps.length<1||e.steps.length>Zr)throw Error(`autograd scenario exceeds the bounded graph size`);let t=new Set;if(e.inputs.forEach((e,n)=>{if(typeof e!=`object`||!e)throw Error(`input must be an object`);if(ni(e.id,`input ${n} id`),ri(e.value,`input ${e.id}`),e.requiresGradient!==!0||t.has(e.id))throw Error(`inputs must require gradients and have unique ids`);t.add(e.id)}),e.steps.forEach((e,n)=>{if(typeof e!=`object`||!e||!Array.isArray(e.inputs))throw Error(`step must contain an inputs array`);if(ni(e.id,`step ${n} id`),t.has(e.id)||![`multiply`,`add`,`square`,`negate`,`branch_nonnegative`].includes(e.operation))throw Error(`step id or operation is invalid`);if(e.inputs.length!==ai(e.operation))throw Error(`${e.operation} has invalid arity`);e.inputs.forEach(n=>{if(ni(n,`step ${e.id} parent`),!t.has(n))throw Error(`step ${e.id} parent must already exist`)}),t.add(e.id)}),e.output!==e.steps.at(-1).id)throw Error(`autograd output must be the final executed step`);let n=new Set(e.inputs.map(e=>e.id)),r=Object.entries(e.mutationsAfterForward);if(r.length>Xr)throw Error(`too many live mutations`);r.forEach(([e,t])=>{if(ni(e,`mutation id`),ri(t,`mutation ${e}`),!n.has(e))throw Error(`mutation ${e} must target an input`)})}function si(e,t={},n=0){let r=[],i=new Map,a=Object.create(null);return e.inputs.forEach(e=>{let a=Object.prototype.hasOwnProperty.call(t,e.id)?t[e.id]:e.value;ri(a,`input ${e.id}`,n);let o={id:e.id,operation:`input`,parents:[],forwardValue:a,savedValues:[]};r.push(o),i.set(o.id,o)}),e.steps.forEach(e=>{let t=e.inputs.map(e=>i.get(e)),n=t.map(e=>e.forwardValue),o=e.operation,s,c=[];e.operation===`multiply`?(s=ti(n[0]*n[1],`${e.id} product`),c=[{name:`left`,sourceId:t[0].id,value:n[0]},{name:`right`,sourceId:t[1].id,value:n[1]}]):e.operation===`add`?s=ti(n[0]+n[1],`${e.id} sum`):e.operation===`square`?(s=ti(n[0]*n[0],`${e.id} square`),c=[{name:`input`,sourceId:t[0].id,value:n[0]}]):e.operation===`negate`?s=ti(-n[0],`${e.id} negation`):n[0]>=0?(o=`identity`,a[e.id]=`nonnegative`,s=n[0]):(o=`negate`,a[e.id]=`negative`,s=ti(-n[0],`${e.id} branch negation`));let l={id:e.id,operation:o,parents:[...e.inputs],forwardValue:s,savedValues:c};r.push(l),i.set(l.id,l)}),{nodes:r,branches:a}}function ci(e,t){let n=new Map(e.map(e=>[e.id,e])),r=new Set,i=[];function a(e){r.has(e)||(r.add(e),n.get(e).parents.forEach(a),i.push(e))}return a(t),i}function li(e,t){let n=e.savedValues.find(e=>e.name===t);if(!n)throw Error(`${e.id} is missing saved ${t}`);return n.value}function ui(e){if(e.operation===`multiply`)return[{parentId:e.parents[0],value:li(e,`right`),source:`saved:right`},{parentId:e.parents[1],value:li(e,`left`),source:`saved:left`}];if(e.operation===`add`)return[{parentId:e.parents[0],value:1,source:`constant:1`},{parentId:e.parents[1],value:1,source:`constant:1`}];if(e.operation===`square`)return[{parentId:e.parents[0],value:ti(2*li(e,`input`),`${e.id} derivative`),source:`saved:input`}];if(e.operation===`negate`)return[{parentId:e.parents[0],value:-1,source:`constant:-1`}];if(e.operation===`identity`)return[{parentId:e.parents[0],value:1,source:`constant:1`}];throw Error(`cannot differentiate ${e.operation}`)}function di(e,t,n){return si(e,t,n).nodes.at(-1).forwardValue}function fi(e,t=1e-5,n=!0){if(oi(e),!Number.isFinite(t)||t<1e-12||t>1)throw Error(`finite-difference epsilon must be finite and in [1e-12, 1]`);let r=ii(e),{nodes:i,branches:a}=si(r),o=new Map(i.map(e=>[e.id,e])),s=ci(i,r.output),c=[...s].reverse(),l=Object.create(null);l[r.output]=1;let u=[];c.forEach(e=>{let t=o.get(e),n=l[e];if(n===void 0||t.operation===`input`)return;let r=ui(t),i=r.map(e=>{let r=ti(n*e.value,`${t.id} parent contribution`);return l[e.parentId]=ti((l[e.parentId]??0)+r,`${e.parentId} accumulated gradient`),{parentId:e.parentId,value:r}});u.push({nodeId:e,operation:t.operation,upstreamGradient:n,localDerivatives:r,parentContributions:i})});let d=Object.fromEntries(r.inputs.map(e=>[e.id,e.value])),f=Object.create(null),p=Object.create(null);return r.inputs.forEach(e=>{let n={...d,[e.id]:e.value+t},i={...d,[e.id]:e.value-t},a=ti((di(r,n,t)-di(r,i,t))/(2*t),`${e.id} finite difference`);f[e.id]=a,p[e.id]=Math.abs(l[e.id]-a)}),{scenario:r,nodes:i,topologicalOrder:s,backwardOrder:c,branchChoices:a,liveInputValues:n?{...d,...r.mutationsAfterForward}:d,backwardSteps:u,gradients:l,finiteDifferenceGradients:f,gradientAbsoluteErrors:p,maxGradientAbsoluteError:Math.max(...Object.values(p),0)}}function pi(e,t=!0){let n=ei.find(t=>t.id===e);if(!n)throw Error(`unknown dynamic autograd scenario: ${e}`);return fi(n,1e-5,t)}function mi(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function hi(e){return e===`input`?`leaf input`:e}function gi(e){return e.operation===`input`?`${e.id} entered the graph as a leaf`:e.operation===`multiply`?`${e.id} = ${e.parents[0]} × ${e.parents[1]}`:e.operation===`add`?`${e.id} = ${e.parents[0]} + ${e.parents[1]}`:e.operation===`square`?`${e.id} = ${e.parents[0]}²`:e.operation===`negate`?`${e.id} = -${e.parents[0]}`:`${e.id} = identity(${e.parents[0]})`}function _i(){let[e,t]=(0,l.useState)(`multiply_add_square`),[n,r]=(0,l.useState)(`m`),[i,a]=(0,l.useState)(0),[o,s]=(0,l.useState)(!0),c=(0,l.useMemo)(()=>pi(e,o),[e,o]),u=c.nodes.find(e=>e.id===n)??c.nodes.at(-1),d=c.backwardSteps[Math.min(i,c.backwardSteps.length-1)],f=Object.keys(c.scenario.mutationsAfterForward).length>0;function p(e){let n=ei.find(t=>t.id===e);t(e),r(n.steps[0].id),a(0),s(!0)}return(0,T.jsxs)(`main`,{className:`workspace workspace--dynamic-autograd`,children:[(0,T.jsxs)(`section`,{className:`autograd-stage`,"aria-label":`Dynamic autograd and saved value visualizer`,children:[(0,T.jsxs)(`section`,{className:`autograd-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN27 / tensor and autograd bridge`}),(0,T.jsx)(`h2`,{children:`Dynamic graph and saved-value microscope`}),(0,T.jsx)(`p`,{children:`The forward run records only executed operations. Backward reverses that graph and reads immutable forward snapshots.`})]}),(0,T.jsx)(`div`,{className:`autograd-chip`,children:`reverse mode`})]}),(0,T.jsxs)(`section`,{className:`autograd-graph-panel`,"aria-label":`Executed dynamic computation graph`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 1 / record what ran`}),(0,T.jsx)(`h2`,{children:c.scenario.expression})]}),(0,T.jsxs)(`span`,{children:[c.nodes.length,` executed nodes`]})]}),(0,T.jsxs)(`div`,{className:`autograd-order-strip`,children:[(0,T.jsx)(`small`,{children:`topological order`}),(0,T.jsx)(`code`,{children:c.topologicalOrder.join(` → `)})]}),(0,T.jsx)(`div`,{className:`autograd-node-lane`,children:c.nodes.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Open node ${e.id}, ${hi(e.operation)}, value ${mi(e.forwardValue)}`,"aria-pressed":e.id===u.id,type:`button`,onClick:()=>r(e.id),children:[(0,T.jsx)(`small`,{children:hi(e.operation)}),(0,T.jsxs)(`strong`,{children:[e.id,` = `,mi(e.forwardValue)]}),(0,T.jsx)(`span`,{children:e.parents.length?`from ${e.parents.join(` + `)}`:`leaf`})]},e.id))}),Object.entries(c.branchChoices).map(([e,t])=>(0,T.jsxs)(`div`,{className:`autograd-branch-note`,children:[(0,T.jsx)(`strong`,{children:e}),` chose the `,(0,T.jsx)(`code`,{children:t}),` branch. The other operation is absent from this graph.`]},e))]}),(0,T.jsxs)(`section`,{className:`autograd-saved-panel`,"aria-label":`Selected node forward and saved value trace`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 2 / save the derivative ingredients`}),(0,T.jsxs)(`h2`,{children:[`Open node `,u.id]})]}),(0,T.jsx)(`span`,{children:hi(u.operation)})]}),(0,T.jsxs)(`div`,{className:`autograd-selected-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`forward rule`}),(0,T.jsx)(`code`,{children:gi(u)}),(0,T.jsxs)(`strong`,{children:[`value `,mi(u.forwardValue)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`saved for backward`}),u.savedValues.length?u.savedValues.map(e=>(0,T.jsxs)(`code`,{children:[e.name,` ← `,e.sourceId,` = `,mi(e.value)]},e.name)):(0,T.jsx)(`code`,{children:`nothing — local derivative is constant`})]})]}),f?(0,T.jsxs)(`div`,{className:`autograd-mutation-strip`,children:[c.scenario.inputs.map(e=>{let t=c.liveInputValues[e.id];return(0,T.jsxs)(`div`,{className:t===e.value?``:`is-mutated`,children:[(0,T.jsx)(`small`,{children:e.id}),(0,T.jsxs)(`code`,{children:[`forward `,mi(e.value)]}),(0,T.jsxs)(`strong`,{children:[`live `,mi(t)]})]},e.id)}),(0,T.jsx)(`p`,{children:`Backward reads the saved forward snapshots, never the later live value.`})]}):null]}),(0,T.jsxs)(`section`,{className:`autograd-backward-panel`,"aria-label":`Reverse topological backward trace`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 3 / reverse the executed graph`}),(0,T.jsx)(`h2`,{children:`Upstream × local derivative`})]}),(0,T.jsx)(`span`,{children:c.backwardOrder.join(` ← `)})]}),(0,T.jsx)(`div`,{className:`autograd-backward-buttons`,children:c.backwardSteps.map((e,t)=>(0,T.jsxs)(`button`,{"aria-label":`Open backward node ${e.nodeId}, upstream ${mi(e.upstreamGradient)}`,"aria-pressed":t===i,type:`button`,onClick:()=>a(t),children:[(0,T.jsx)(`small`,{children:e.operation}),(0,T.jsx)(`strong`,{children:e.nodeId}),(0,T.jsxs)(`code`,{children:[`upstream `,mi(e.upstreamGradient)]})]},e.nodeId))}),(0,T.jsx)(`div`,{className:`autograd-backward-equations`,"aria-label":`Selected backward calculation`,children:d.localDerivatives.map((e,t)=>{let n=d.parentContributions[t];return(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`toward `,e.parentId]}),(0,T.jsxs)(`code`,{children:[mi(d.upstreamGradient),` × `,mi(e.value),` = `,mi(n.value)]}),(0,T.jsxs)(`span`,{children:[`local source: `,e.source]})]},`${d.nodeId}-${e.parentId}`)})})]}),(0,T.jsxs)(`section`,{className:`autograd-audit-panel`,"aria-label":`Dynamic autograd finite difference audit`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 4 / distrust the graph once`}),(0,T.jsx)(`h2`,{children:`Fresh forwards check every leaf`})]}),(0,T.jsx)(`span`,{children:`epsilon 1e-5`})]}),(0,T.jsxs)(`div`,{className:`autograd-audit-grid`,children:[c.scenario.inputs.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`strong`,{children:e.id}),(0,T.jsxs)(`span`,{children:[`analytical `,(0,T.jsx)(`code`,{children:mi(c.gradients[e.id])})]}),(0,T.jsxs)(`span`,{children:[`numerical `,(0,T.jsx)(`code`,{children:mi(c.finiteDifferenceGradients[e.id])})]}),(0,T.jsxs)(`small`,{children:[`error `,mi(c.gradientAbsoluteErrors[e.id])]})]},e.id)),(0,T.jsxs)(`div`,{className:`autograd-audit-max`,children:[(0,T.jsx)(`strong`,{children:`maximum error`}),(0,T.jsx)(`code`,{children:mi(c.maxGradientAbsoluteError)}),(0,T.jsx)(`small`,{children:`must stay below 1e-8`})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`controls autograd-controls`,"aria-label":`Dynamic autograd scenarios`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Graph presets`}),(0,T.jsx)(`h2`,{children:`Change one graph rule`}),(0,T.jsx)(`div`,{className:`autograd-scenario-buttons`,children:ei.map(t=>(0,T.jsxs)(`button`,{"aria-pressed":t.id===e,type:`button`,onClick:()=>p(t.id),children:[(0,T.jsx)(`strong`,{children:t.title}),(0,T.jsx)(`code`,{children:t.expression}),(0,T.jsx)(`span`,{children:t.summary})]},t.id))}),f?(0,T.jsx)(`button`,{className:`autograd-mutation-toggle`,"aria-pressed":o,type:`button`,onClick:()=>s(e=>!e),children:o?`Restore forward-time live values`:`Apply post-forward mutation`}):null,(0,T.jsxs)(`div`,{className:`autograd-mental-model`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Keep this picture`}),(0,T.jsx)(`h2`,{children:`Record, save, reverse.`}),(0,T.jsx)(`p`,{children:`Record only executed operations. Save only derivative ingredients. Reverse children before parents.`})]})]})]})}var vi=/^[A-Za-z][A-Za-z0-9_]{0,63}$/,yi=8,bi=1e3,xi=0xe8d4a51000,Si=[{id:`single_row`,title:`One row by hand`,summary:`Follow 4 and 8 through every graph, NeuralIR, and MatrixIR value.`,inputs:{x0:[4],x1:[8]}},{id:`two_row_batch`,title:`The same plan, two rows`,summary:`Keep the lowered program fixed while its input columns grow to two rows.`,inputs:{x0:[4,8],x1:[8,16]}}],Ci=[{id:`x0`,op:`input`,detail:`runtime input x0`},{id:`x1`,op:`input`,detail:`runtime input x1`},{id:`bias`,op:`constant`,detail:`constant 1`},{id:`sum`,op:`weighted_sum`,detail:`three weighted terms`},{id:`relu`,op:`activation`,detail:`max(0, sum)`},{id:`out`,op:`output`,detail:`prediction`}],wi=[{id:`w0`,from:`x0`,to:`sum`,weight:.25},{id:`w1`,from:`x1`,to:`sum`,weight:.75},{id:`bias_to_sum`,from:`bias`,to:`sum`,weight:-1},{id:`sum_to_relu`,from:`sum`,to:`relu`,weight:1},{id:`relu_to_out`,from:`relu`,to:`out`,weight:1}];function Ti(e,t,n=512){if(typeof e!=`string`||e.length<1||e.length>n)throw Error(`${t} must be a bounded string`);return e}function Ei(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>bi)throw Error(`${t} must be finite and bounded`);return e}function Di(e,t){if(!Number.isFinite(e)||Math.abs(e)>xi)throw Error(`${t} must remain finite and bounded`);return e}function Oi(e){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`forward lowering scenario must be an object`);if(Object.keys(e).sort().join(`,`)!==`id,inputs,summary,title`)throw Error(`forward lowering scenario has an unexpected field`);let t=Ti(e.id,`scenario id`,64);if(!vi.test(t))throw Error(`scenario id must be a bounded identifier`);let n=Ti(e.title,`scenario title`),r=Ti(e.summary,`scenario summary`);if(typeof e.inputs!=`object`||e.inputs===null||Array.isArray(e.inputs))throw Error(`scenario inputs must be an object`);if(Object.keys(e.inputs).sort().join(`,`)!==`x0,x1`)throw Error(`scenario inputs must contain exactly x0 and x1`);let i=e.inputs.x0,a=e.inputs.x1;if(!Array.isArray(i)||!Array.isArray(a))throw Error(`scenario input columns must be arrays`);if(i.length<1||i.length>yi||a.length!==i.length)throw Error(`scenario input columns must have the same bounded length`);return zi({id:t,title:n,summary:r,inputs:{x0:i.map((e,t)=>Ei(e,`x0[${t}]`)),x1:a.map((e,t)=>Ei(e,`x1[${t}]`))}})}function ki(){let e=$e(`tiny-weighted-relu`);return et(e,`x0`),et(e,`x1`),tt(e,`bias`,1),nt(e,`sum`,[{from:`x0`,weight:.25,edgeId:`w0`},{from:`x1`,weight:.75,edgeId:`w1`},{from:`bias`,weight:-1,edgeId:`bias_to_sum`}]),rt(e,`relu`,`sum`,`relu`,{},`sum_to_relu`),it(e,`out`,`relu`,`prediction`,{},`relu_to_out`),e}function Ai(e){return e.op===`MUL`?[e.left,e.right]:e.op===`ADD`?[...e.inputs??[]]:e.op===`ACTIVATE`||e.op===`STORE_OUTPUT`?[e.input]:[]}function ji(e){switch(e.op){case`LOAD_INPUT`:return{input_name:e.inputName};case`LOAD_CONST`:return{value:e.value??0};case`LOAD_EDGE_WEIGHT`:return{edge_id:e.edgeId};case`ACTIVATE`:return{activation:e.activation??`relu`};case`STORE_OUTPUT`:return{output_name:e.outputName??`output`};default:return{}}}function Mi(e){return e.map((e,t)=>({id:`i${t}`,op:e.op,output:e.dst??null,inputs:Ai(e),attributes:ji(e),sourceNodes:e.sourceNode===void 0?[]:[e.sourceNode],sourceEdges:e.sourceEdge===void 0?[]:[e.sourceEdge]}))}function Ni(e,t){let n=new Set((e.terms??[]).map(e=>e.edgeId));return t.filter(t=>t.sourceEdges.some(e=>n.has(e))||t.op===`ADD`&&t.output===e.dst).map(e=>e.id)}function Pi(e,t){return e.map((e,n)=>{let r=e.op===`WEIGHTED_SUM_MATRIX`,i=e.terms??[],a=r?i.map(e=>e.sourceValue):e.input===void 0?[]:[e.input],o={};return e.op===`LOAD_INPUT_MATRIX`&&(o.input_name=e.inputName),e.op===`LOAD_CONST_MATRIX`&&(o.value=e.value??0),r&&(o.edge_ids=i.map(e=>e.edgeId),o.weights=i.map(e=>e.weight)),e.op===`ACTIVATE_MATRIX`&&(o.activation=e.activation??`relu`),e.op===`STORE_OUTPUT_MATRIX`&&(o.output_name=e.outputName??`output`),{id:`m${n}`,op:e.op,output:e.dst??null,inputs:a,attributes:o,sourceInstructions:r?Ni(e,t):e.sourceInstructionIndexes.map(e=>`i${e}`),sourceNodes:e.sourceNode===void 0?[]:[e.sourceNode],sourceEdges:r?i.map(e=>e.edgeId):[]}})}function Fi(e){return e.inputs.x0.map((t,n)=>{let r=Di(Di(-1,`bias term`)+Di(t*.25,`x0 term`)+Di(e.inputs.x1[n]*.75,`x1 term`),`direct row ${n}`);return Math.max(0,r)})}function Ii(e){let t=0;for(let n=0;n<e[0].length;n+=1)for(let r=0;r<e.length;r+=1)for(let i=r+1;i<e.length;i+=1)t=Math.max(t,Math.abs(e[r][n]-e[i][n]));return Di(t,`parity error`)}function Li(e){let t=Oi(e),n=st(ki()),r=n.functions[0],i=wt(n),a=Mi(r.instructions),o=Pi(i.instructions,a),s=t.inputs.x0.map((e,r)=>lt(n,{x0:e,x1:t.inputs.x1[r]})),c=s.map(e=>Di(e.outputs.prediction,`NeuralIR output`)),l=s.map(e=>Object.values(e.values).map(e=>Di(e,`NeuralIR value`))),u=Et(i,t.inputs),d=(u.outputs.prediction??[]).map(e=>Di(e,`MatrixIR output`)),f=Object.entries(u.values).map(([e,t])=>({valueId:e,values:t.map(t=>Di(t,`MatrixIR ${e}`))})),p=Fi(t),m=Ii([p,c,d]),h=s[0].instructions.map((e,t)=>({instructionId:`i${t}`,reads:e.reads.map(e=>({...e})),write:e.write===void 0?void 0:{...e.write},output:e.output===void 0?void 0:{...e.output}}));return zi({scenario:t,graph:{nodes:Ci.map(e=>({...e})),edges:wi.map(e=>({...e})),topologicalOrder:[`bias`,`x0`,`x1`,`sum`,`relu`,`out`]},neuralIr:{magic:`CANN`,version:0,instructions:a},matrixIr:{magic:`CANM`,version:0,sourceNeuralIrVersion:0,operations:o},directOutputs:p,neuralIrOutputs:c,matrixIrOutputs:d,neuralValueRows:l,matrixValueColumns:f,firstRowInstructionReadings:h,maxParityError:m})}function Ri(e){let t=Si.find(t=>t.id===e);if(t===void 0)throw Error(`unknown forward lowering scenario: ${e}`);return Li(t)}function zi(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>zi(e)),e)}function Bi(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(10)).toString()}function Vi(e){switch(e.op){case`LOAD_CONST`:return`materialize ${e.attributes.value}`;case`LOAD_INPUT`:return`bind ${e.attributes.input_name}`;case`LOAD_EDGE_WEIGHT`:return`load ${e.attributes.edge_id}`;case`MUL`:return`${e.inputs.join(` x `)}`;case`ADD`:return e.inputs.join(` + `);case`ACTIVATE`:return`${e.attributes.activation}(${e.inputs[0]})`;case`STORE_OUTPUT`:return`publish ${e.attributes.output_name}`;default:return e.op}}function Hi(e){switch(e.op){case`LOAD_CONST_MATRIX`:return`broadcast ${e.attributes.value}`;case`LOAD_INPUT_MATRIX`:return`column ${e.attributes.input_name}`;case`WEIGHTED_SUM_MATRIX`:return`${e.inputs.length} fused terms`;case`ACTIVATE_MATRIX`:return`${e.attributes.activation} column`;case`STORE_OUTPUT_MATRIX`:return`publish ${e.attributes.output_name}`;default:return e.op}}function j(e){let t=Object.entries(e);return t.length===0?`none`:t.map(([e,t])=>`${e}=${Array.isArray(t)?`[${t.join(`, `)}]`:String(t)}`).join(`; `)}function Ui(){let[e,t]=(0,l.useState)(`single_row`),[n,r]=(0,l.useState)({lane:`matrix`,id:`m3`}),i=(0,l.useMemo)(()=>Ri(e),[e]),a=n.lane===`neural`?i.neuralIr.instructions.find(e=>e.id===n.id):void 0,o=n.lane===`matrix`?i.matrixIr.operations.find(e=>e.id===n.id):void 0,s=a===void 0?void 0:i.firstRowInstructionReadings.find(e=>e.instructionId===a.id);return(0,T.jsxs)(`main`,{className:`workspace workspace--forward-lowering`,children:[(0,T.jsxs)(`section`,{className:`forward-lowering-stage`,children:[(0,T.jsxs)(`header`,{className:`forward-lowering-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN29 - graph -> NeuralIR -> MatrixIR`}),(0,T.jsx)(`h2`,{children:`Forward graph lowering map`}),(0,T.jsx)(`p`,{children:`Keep one prediction fixed while a dependency graph becomes an ordered scalar program and then a fused batch plan.`})]}),(0,T.jsxs)(`span`,{className:`forward-lowering-chip`,children:[`6 nodes -> `,i.neuralIr.instructions.length,` instructions -> `,i.matrixIr.operations.length,` ops`]})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-graph`,"aria-label":`Canonical forward neural graph`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`1 - meaning`}),(0,T.jsx)(`h2`,{children:`The graph says what depends on what`})]}),(0,T.jsx)(`code`,{children:i.graph.topologicalOrder.join(` -> `)})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-node-flow`,children:[(0,T.jsx)(`div`,{className:`forward-lowering-input-stack`,children:i.graph.nodes.slice(0,3).map(e=>(0,T.jsxs)(`article`,{children:[(0,T.jsx)(`strong`,{children:e.id}),(0,T.jsx)(`span`,{children:e.detail})]},e.id))}),(0,T.jsx)(`span`,{className:`forward-lowering-arrow`,children:`->`}),i.graph.nodes.slice(3).map((e,t)=>(0,T.jsxs)(`div`,{className:`forward-lowering-flow-tail`,children:[(0,T.jsxs)(`article`,{children:[(0,T.jsx)(`strong`,{children:e.id}),(0,T.jsx)(`span`,{children:e.detail})]}),t<2?(0,T.jsx)(`span`,{className:`forward-lowering-arrow`,children:`->`}):null]},e.id))]}),(0,T.jsx)(`div`,{className:`forward-lowering-edge-grid`,children:i.graph.edges.slice(0,3).map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`code`,{children:e.id}),(0,T.jsxs)(`span`,{children:[e.from,` -> `,e.to]}),(0,T.jsxs)(`strong`,{children:[`x `,Bi(e.weight)]})]},e.id))})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`NeuralIR instruction stream`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`2 - schedule`}),(0,T.jsx)(`h2`,{children:`NeuralIR writes each value once`})]}),(0,T.jsxs)(`code`,{children:[i.neuralIr.magic,` v`,i.neuralIr.version]})]}),(0,T.jsx)(`div`,{className:`forward-lowering-instruction-lane`,children:i.neuralIr.instructions.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Open NeuralIR ${e.id}, ${e.op}`,"aria-pressed":n.lane===`neural`&&n.id===e.id,onClick:()=>r({lane:`neural`,id:e.id}),type:`button`,children:[(0,T.jsx)(`small`,{children:e.id}),(0,T.jsx)(`strong`,{children:e.op}),(0,T.jsx)(`code`,{children:e.output??`output boundary`}),(0,T.jsx)(`span`,{children:Vi(e)})]},e.id))})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`MatrixIR operation stream`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`3 - fuse`}),(0,T.jsx)(`h2`,{children:`MatrixIR keeps columns together`})]}),(0,T.jsxs)(`code`,{children:[i.matrixIr.magic,` v`,i.matrixIr.version]})]}),(0,T.jsx)(`div`,{className:`forward-lowering-matrix-lane`,children:i.matrixIr.operations.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Open MatrixIR ${e.id}, ${e.op}`,"aria-pressed":n.lane===`matrix`&&n.id===e.id,onClick:()=>r({lane:`matrix`,id:e.id}),type:`button`,children:[(0,T.jsx)(`small`,{children:e.id}),(0,T.jsx)(`strong`,{children:e.op}),(0,T.jsx)(`code`,{children:e.output??`output boundary`}),(0,T.jsx)(`span`,{children:Hi(e)})]},e.id))})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-selection`,"aria-label":`Selected lowering detail`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`selected translation`}),(0,T.jsx)(`h2`,{children:a?.op??o?.op})]}),(0,T.jsx)(`code`,{children:n.id})]}),a===void 0?o===void 0?null:(0,T.jsxs)(`div`,{className:`forward-lowering-detail-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`fuses NeuralIR`}),(0,T.jsx)(`code`,{children:o.sourceInstructions.join(`, `)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`attributes`}),(0,T.jsx)(`code`,{children:j(o.attributes)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`graph provenance`}),(0,T.jsx)(`code`,{children:[...o.sourceNodes,...o.sourceEdges].join(`, `)||`none`})]})]}):(0,T.jsxs)(`div`,{className:`forward-lowering-detail-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`reads`}),(0,T.jsx)(`code`,{children:s?.reads.map(e=>`${e.valueId}=${Bi(e.value)}`).join(`, `)||`none`})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`writes`}),(0,T.jsx)(`code`,{children:s?.write===void 0?`${s?.output?.outputName}=${Bi(s?.output?.value??0)}`:`${s.write.valueId}=${Bi(s.write.value)}`})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`graph provenance`}),(0,T.jsx)(`code`,{children:[...a.sourceNodes,...a.sourceEdges].join(`, `)||`none`})]})]})]}),(0,T.jsxs)(`section`,{className:`forward-lowering-parity`,"aria-label":`Forward lowering execution parity`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`4 - prove equivalence`}),(0,T.jsx)(`h2`,{children:`Three paths, the same prediction`})]}),(0,T.jsxs)(`code`,{children:[`max error `,i.maxParityError.toExponential(1)]})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-parity-table`,role:`table`,"aria-label":`Direct NeuralIR MatrixIR outputs`,children:[(0,T.jsxs)(`div`,{className:`forward-lowering-parity-head`,role:`row`,children:[(0,T.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`x0`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`x1`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`direct`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`NeuralIR`}),(0,T.jsx)(`strong`,{role:`columnheader`,children:`MatrixIR`})]}),i.directOutputs.map((e,t)=>(0,T.jsxs)(`div`,{role:`row`,children:[(0,T.jsx)(`strong`,{role:`cell`,children:t}),(0,T.jsx)(`code`,{role:`cell`,children:Bi(i.scenario.inputs.x0[t])}),(0,T.jsx)(`code`,{role:`cell`,children:Bi(i.scenario.inputs.x1[t])}),(0,T.jsx)(`code`,{role:`cell`,children:Bi(e)}),(0,T.jsx)(`code`,{role:`cell`,children:Bi(i.neuralIrOutputs[t])}),(0,T.jsx)(`code`,{role:`cell`,children:Bi(i.matrixIrOutputs[t])})]},t))]})]})]}),(0,T.jsxs)(`aside`,{className:`forward-lowering-controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Run shape`}),(0,T.jsx)(`h2`,{children:`Keep the compiler fixed`}),(0,T.jsx)(`p`,{children:`Change only the number of input rows and watch every IR identifier stay stable.`}),(0,T.jsx)(`div`,{className:`forward-lowering-scenario-buttons`,children:Si.map(n=>(0,T.jsxs)(`button`,{"aria-label":n.title,"aria-pressed":e===n.id,onClick:()=>t(n.id),type:`button`,children:[(0,T.jsx)(`strong`,{children:n.title}),(0,T.jsx)(`span`,{children:n.summary})]},n.id))}),(0,T.jsxs)(`div`,{className:`forward-lowering-equation`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Paper result`}),(0,T.jsx)(`code`,{children:`z = -1 + 0.25x0 + 0.75x1`}),(0,T.jsx)(`code`,{children:`prediction = max(0, z)`})]}),(0,T.jsxs)(`div`,{className:`forward-lowering-mental-model`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Rust boundary`}),(0,T.jsx)(`h2`,{children:`Meaning stays above tensors`}),(0,T.jsx)(`p`,{children:`The neural compiler retains source IDs and fusion rules. A Rust MX01 bridge receives explicit tensors, dtypes, shapes, constants, inputs, and outputs.`})]})]})]})}var Wi=/^[a-z][a-z0-9_]{0,31}$/,Gi=4,Ki=12,qi=1e3,Ji=0xe8d4a51000,Yi=1e-5,Xi=[{id:`a`,input:2,target:1},{id:`b`,input:-1,target:1}],Zi=[{id:`accumulate_two_calls`,title:`Two backward calls`,summary:`The second backward adds 2 to the 2 already in w.grad.`,initialParameter:1,learningRate:.1,samples:Xi,events:[{kind:`backward`,sampleId:`a`},{kind:`backward`,sampleId:`b`}]},{id:`zero_between_calls`,title:`Zero between calls`,summary:`Clearing the buffer makes the second gradient stand alone.`,initialParameter:1,learningRate:.1,samples:Xi,events:[{kind:`backward`,sampleId:`a`},{kind:`zero_grad`},{kind:`backward`,sampleId:`b`}]},{id:`mean_then_zero`,title:`Mean, step, zero`,summary:`Two micro-batches become one mean gradient, then zero starts the next batch clean.`,initialParameter:1,learningRate:.1,samples:Xi,events:[{kind:`backward`,sampleId:`a`},{kind:`backward`,sampleId:`b`},{kind:`optimizer_step`,divisor:2},{kind:`zero_grad`}]},{id:`stale_next_batch`,title:`Forgotten zero`,summary:`A new 0.8 gradient lands on a stale buffer of 4 and drives the wrong update.`,initialParameter:1,learningRate:.1,samples:[...Xi,{id:`c`,input:1,target:0}],events:[{kind:`backward`,sampleId:`a`},{kind:`backward`,sampleId:`b`},{kind:`optimizer_step`,divisor:2},{kind:`backward`,sampleId:`c`},{kind:`optimizer_step`,divisor:1}]}];function Qi(e,t){if(!Number.isFinite(e)||Math.abs(e)>Ji)throw Error(`${t} must remain finite and bounded`);return e}function $i(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>qi)throw Error(`${t} must be finite and bounded`)}function ea(e){if(typeof e!=`object`||!e||!Array.isArray(e.samples)||!Array.isArray(e.events))throw Error(`gradient schedule must contain bounded sample and event arrays`);if(typeof e.id!=`string`||!Wi.test(e.id)||typeof e.title!=`string`||e.title.length<1||e.title.length>256||typeof e.summary!=`string`||e.summary.length<1||e.summary.length>512)throw Error(`gradient schedule metadata must contain bounded strings`);if(e.samples.length<1||e.samples.length>Gi||e.events.length<1||e.events.length>Ki)throw Error(`gradient schedule exceeds bounded sizes`);if($i(e.initialParameter,`initial parameter`),$i(e.learningRate,`learning rate`),e.learningRate<=0||e.learningRate>1)throw Error(`learning rate must be in (0, 1]`);let t=new Set;e.samples.forEach(e=>{if(typeof e!=`object`||!e||typeof e.id!=`string`||!Wi.test(e.id))throw Error(`sample must have a bounded identifier`);if(t.has(e.id))throw Error(`duplicate sample id ${e.id}`);$i(e.input,`sample ${e.id} input`),$i(e.target,`sample ${e.id} target`),t.add(e.id)});let n=0;if(e.events.forEach(e=>{if(typeof e!=`object`||!e)throw Error(`event must be an object`);if(e.kind===`backward`){if(typeof e.sampleId!=`string`||!Wi.test(e.sampleId))throw Error(`backward sample id must be a bounded identifier`);if(!t.has(e.sampleId))throw Error(`backward references unknown sample ${e.sampleId}`);n+=1}else if(e.kind===`optimizer_step`){if(!Number.isInteger(e.divisor)||e.divisor<1||e.divisor>Gi)throw Error(`optimizer divisor must be a bounded positive integer`)}else if(e.kind!==`zero_grad`)throw Error(`unsupported gradient schedule event`)}),n===0)throw Error(`gradient schedule needs a backward call`)}function ta(e){let t=e.samples.map(e=>Object.freeze({...e})),n=e.events.map(e=>Object.freeze({...e}));return Object.freeze({id:e.id,title:e.title,summary:e.summary,initialParameter:e.initialParameter,learningRate:e.learningRate,samples:Object.freeze(t),events:Object.freeze(n)})}function na(e,t){let n=Qi(Qi(e*t.input,`finite-difference prediction`)-t.target,`finite-difference residual`);return Qi(.5*n*n,`finite-difference loss`)}function ra(e,t=Yi){if(ea(e),!Number.isFinite(t)||t<1e-12||t>1)throw Error(`finite-difference epsilon must be in [1e-12, 1]`);let n=ta(e),r=new Map(n.samples.map(e=>[e.id,e])),i=[],a=n.initialParameter,o=0,s=0,c=0,l=0,u=0;return n.events.forEach((e,d)=>{let f=a,p=o;if(e.kind===`backward`){let n=r.get(e.sampleId),c=Qi(a*n.input,`event ${d} prediction`),l=Qi(c-n.target,`event ${d} residual`),m=Qi(.5*l*l,`event ${d} loss`),h=Qi(l*n.input,`event ${d} gradient`);o=Qi(o+h,`event ${d} buffer`);let g=Qi((na(a+t,n)-na(a-t,n))/(2*t),`event ${d} numerical gradient`),_=Math.abs(h-g);u=Math.max(u,_),s+=1,i.push({index:d,kind:`backward`,sampleId:n.id,input:n.input,target:n.target,parameterBefore:f,parameterAfter:a,bufferBefore:p,bufferAfter:o,prediction:c,residual:l,loss:m,localGradient:h,numericalGradient:g,gradientAbsoluteError:_})}else if(e.kind===`zero_grad`)o=0,l+=1,i.push({index:d,kind:`zero_grad`,parameterBefore:f,parameterAfter:a,bufferBefore:p,bufferAfter:o});else{let t=Qi(o/e.divisor,`event ${d} applied gradient`),r=Qi(-n.learningRate*t,`event ${d} parameter delta`);a=Qi(a+r,`event ${d} parameter`),c+=1,i.push({index:d,kind:`optimizer_step`,parameterBefore:f,parameterAfter:a,bufferBefore:p,bufferAfter:o,divisor:e.divisor,appliedGradient:t,parameterDelta:r})}}),{scenario:n,steps:i,finalParameter:a,finalGradientBuffer:o,backwardCalls:s,optimizerSteps:c,zeroCalls:l,maxGradientAbsoluteError:u}}function ia(e){let t=Zi.find(t=>t.id===e);if(!t)throw Error(`unknown gradient accumulation scenario: ${e}`);return ra(t)}function M(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function aa(e){return e.kind===`backward`?`backward(${e.sampleId})`:e.kind===`zero_grad`?`zero_grad()`:`step(grad / ${e.divisor})`}function oa(){let[e,t]=(0,l.useState)(`accumulate_two_calls`),[n,r]=(0,l.useState)(0),i=(0,l.useMemo)(()=>ia(e),[e]),a=i.steps[Math.min(n,i.steps.length-1)];function o(e){t(e),r(0)}return(0,T.jsxs)(`main`,{className:`workspace workspace--gradient-buffer`,children:[(0,T.jsxs)(`section`,{className:`gradient-buffer-stage`,"aria-label":`Gradient accumulation and zeroing visualizer`,children:[(0,T.jsxs)(`section`,{className:`gradient-buffer-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN28 / tensor and autograd bridge`}),(0,T.jsx)(`h2`,{children:`Gradient buffer timeline`}),(0,T.jsx)(`p`,{children:`Backward adds into a persistent buffer. An optimizer reads it, but only an explicit zero clears it.`})]}),(0,T.jsx)(`div`,{className:`gradient-buffer-chip`,children:`w.grad += local`})]}),(0,T.jsxs)(`section`,{className:`gradient-buffer-state`,"aria-label":`Selected gradient buffer state`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`The two pieces of mutable state`}),(0,T.jsx)(`h2`,{children:`Parameter and gradient buffer`})]}),(0,T.jsxs)(`span`,{children:[`event `,a.index+1,` of `,i.steps.length]})]}),(0,T.jsxs)(`div`,{className:`gradient-buffer-vessels`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`parameter w`}),(0,T.jsx)(`code`,{children:M(a.parameterBefore)}),(0,T.jsx)(`span`,{children:`→`}),(0,T.jsx)(`strong`,{children:M(a.parameterAfter)})]}),(0,T.jsxs)(`div`,{className:a.bufferAfter===0?`is-empty`:`is-filled`,children:[(0,T.jsx)(`small`,{children:`persistent w.grad`}),(0,T.jsx)(`code`,{children:M(a.bufferBefore)}),(0,T.jsx)(`span`,{children:`→`}),(0,T.jsx)(`strong`,{children:M(a.bufferAfter)})]})]})]}),(0,T.jsxs)(`section`,{className:`gradient-buffer-timeline`,"aria-label":`Gradient schedule timeline`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Replay the schedule`}),(0,T.jsx)(`h2`,{children:`Every API call is a state transition`})]}),(0,T.jsxs)(`span`,{children:[i.backwardCalls,` backward / `,i.optimizerSteps,` step / `,i.zeroCalls,` zero`]})]}),(0,T.jsx)(`div`,{className:`gradient-buffer-event-lane`,children:i.steps.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Open event ${e.index+1}, ${aa(e)}, buffer ${M(e.bufferBefore)} to ${M(e.bufferAfter)}`,"aria-pressed":e.index===a.index,type:`button`,onClick:()=>r(e.index),children:[(0,T.jsxs)(`small`,{children:[`event `,e.index+1]}),(0,T.jsx)(`strong`,{children:aa(e)}),(0,T.jsxs)(`code`,{children:[`grad `,M(e.bufferBefore),` → `,M(e.bufferAfter)]})]},e.index))})]}),(0,T.jsxs)(`section`,{className:`gradient-buffer-equation`,"aria-label":`Selected gradient buffer calculation`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Open the arithmetic`}),(0,T.jsx)(`h2`,{children:aa(a)})]}),(0,T.jsx)(`span`,{children:a.kind.replace(`_`,` `)})]}),a.kind===`backward`?(0,T.jsxs)(`div`,{className:`gradient-buffer-backward-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`forward sample `,a.sampleId]}),(0,T.jsxs)(`code`,{children:[M(a.parameterBefore),` × `,M(a.input),` = `,M(a.prediction)]}),(0,T.jsxs)(`code`,{children:[M(a.prediction),` - `,M(a.target),` = `,M(a.residual)]}),(0,T.jsxs)(`strong`,{children:[`½ × `,M(a.residual),`² = `,M(a.loss)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`local gradient`}),(0,T.jsxs)(`code`,{children:[`(`,M(a.prediction),` - `,M(a.target),`) × `,M(a.input)]}),(0,T.jsxs)(`strong`,{children:[`dL/dw = `,M(a.localGradient)]})]}),(0,T.jsxs)(`div`,{className:`gradient-buffer-addition`,children:[(0,T.jsx)(`small`,{children:`buffer addition`}),(0,T.jsxs)(`code`,{children:[M(a.bufferBefore),` + `,M(a.localGradient)]}),(0,T.jsxs)(`strong`,{children:[`w.grad = `,M(a.bufferAfter)]})]})]}):a.kind===`zero_grad`?(0,T.jsxs)(`div`,{className:`gradient-buffer-zero-rule`,children:[(0,T.jsx)(`code`,{children:`w.grad ← 0`}),(0,T.jsxs)(`p`,{children:[`The parameter stays `,M(a.parameterAfter),`. Only the buffer is cleared.`]})]}):(0,T.jsxs)(`div`,{className:`gradient-buffer-step-rule`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`choose sum or mean`}),(0,T.jsxs)(`code`,{children:[M(a.bufferBefore),` / `,a.divisor,` = `,M(a.appliedGradient)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`SGD update`}),(0,T.jsxs)(`code`,{children:[M(a.parameterBefore),` - `,M(i.scenario.learningRate),` × `,M(a.appliedGradient)]}),(0,T.jsxs)(`strong`,{children:[`w = `,M(a.parameterAfter)]})]}),(0,T.jsxs)(`p`,{children:[`The optimizer read `,M(a.bufferBefore),` but left that buffer unchanged.`]})]})]}),(0,T.jsxs)(`section`,{className:`gradient-buffer-audit`,"aria-label":`Gradient buffer numerical audit`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Independent check`}),(0,T.jsx)(`h2`,{children:`Each local gradient gets fresh forward passes`})]}),(0,T.jsx)(`span`,{children:`epsilon 1e-5`})]}),(0,T.jsxs)(`div`,{className:`gradient-buffer-audit-grid`,children:[i.steps.filter(e=>e.kind===`backward`).map(e=>e.kind===`backward`?(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`strong`,{children:[`event `,e.index+1,` / `,e.sampleId]}),(0,T.jsxs)(`span`,{children:[`analytical `,(0,T.jsx)(`code`,{children:M(e.localGradient)})]}),(0,T.jsxs)(`span`,{children:[`numerical `,(0,T.jsx)(`code`,{children:M(e.numericalGradient)})]}),(0,T.jsxs)(`small`,{children:[`error `,M(e.gradientAbsoluteError)]})]},e.index):null),(0,T.jsxs)(`div`,{className:`gradient-buffer-audit-max`,children:[(0,T.jsx)(`strong`,{children:`maximum error`}),(0,T.jsx)(`code`,{children:M(i.maxGradientAbsoluteError)}),(0,T.jsx)(`small`,{children:`must stay below 1e-8`})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`controls gradient-buffer-controls`,"aria-label":`Gradient buffer scenarios`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Schedule presets`}),(0,T.jsx)(`h2`,{children:`Move the zero call`}),(0,T.jsx)(`div`,{className:`gradient-buffer-scenario-buttons`,children:Zi.map(t=>(0,T.jsxs)(`button`,{"aria-pressed":t.id===e,type:`button`,onClick:()=>o(t.id),children:[(0,T.jsx)(`strong`,{children:t.title}),(0,T.jsx)(`span`,{children:t.summary})]},t.id))}),(0,T.jsxs)(`div`,{className:`gradient-buffer-summary`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Final state`}),(0,T.jsxs)(`code`,{children:[`w = `,M(i.finalParameter)]}),(0,T.jsxs)(`code`,{children:[`w.grad = `,M(i.finalGradientBuffer)]})]}),(0,T.jsxs)(`div`,{className:`gradient-buffer-mental-model`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Keep this picture`}),(0,T.jsx)(`h2`,{children:`Backward adds. Step reads. Zero clears.`}),(0,T.jsx)(`p`,{children:`Accumulation is useful across micro-batches and dangerous across optimizer steps unless the schedule is deliberate.`})]})]})]})}function sa(e){return e.map(e=>[...e])}function ca(e){return{name:e.name,weights:sa(e.weights),biases:[...e.biases],activation:e.activation}}function la(e,t){if(t.length===0||t[0]===void 0||t[0].length===0)throw Error(`${e} must have at least one row and one column`);let n=t[0].length;for(let r of t)if(r.length!==n)throw Error(`${e} must be rectangular`);return{rows:t.length,cols:n}}function ua(e){let t=e>>>0;return()=>(t=Math.imul(1664525,t)+1013904223>>>0,t/4294967296)}function da(e,t){return(e()*2-1)*t}function fa(e,t){return e.map(e=>e.map((e,n)=>e+t[n]))}function pa(e){let t=e[0]?.length??0,n=Array(t).fill(0);for(let r of e)for(let e=0;e<t;e+=1)n[e]+=r[e];return n}function ma(e){let t=0,n=0;for(let r of e)for(let e of r)t+=e*e,n+=1;return n===0?0:t/n}function ha(e){return e.length===0?0:e.reduce((e,t)=>e+t*t,0)/e.length}function ga(e,t){switch(t){case`linear`:return e;case`sigmoid`:if(e>=0)return 1/(1+Math.exp(-e));{let t=Math.exp(e);return t/(1+t)}case`tanh`:return Math.tanh(e);case`relu`:return Math.max(0,e)}}function _a(e,t,n){switch(n){case`linear`:return 1;case`sigmoid`:return t*(1-t);case`tanh`:return 1-t*t;case`relu`:return+(e>0)}}function va(e,t){return e.map(e=>e.map(e=>ga(e,t)))}function ya(e,t,n){return e.map((e,r)=>e.map((e,i)=>_a(e,t[r][i],n)))}function ba(e,t){return e.map((e,n)=>e.map((e,r)=>e*t[n][r]))}function xa(e,t,n){return new xt(e).subtract(new xt(t).scale(n)).data}function Sa(e,t,n){return e.map((e,r)=>e-n*t[r])}function Ca(e,t,n){let r=la(`${e.name} weights`,e.weights);if(r.rows!==t)throw Error(`${e.name} weight row count must match previous layer width`);if(r.cols!==e.biases.length)throw Error(`${e.name} weight columns must match bias count`);if(e.biases.length===0)throw Error(`${e.name} must have at least one neuron`);for(let t of e.weights)for(let n of t)if(!Number.isFinite(n))throw Error(`${e.name} weights must be finite`);for(let t of e.biases)if(!Number.isFinite(t))throw Error(`${e.name} biases must be finite`);if(n<0)throw Error(`layer index must be non-negative`)}function wa(e,t,n,r,i=7,a=1){if(n<1)throw Error(`hiddenLayerCount must be at least one`);let o=ua(i),s=[],c=e;for(let e=0;e<n;e+=1){let r=n===1?a:a/Math.sqrt(Math.max(1,c));s.push({name:`hidden${e+1}`,weights:Array.from({length:c},()=>Array.from({length:t},()=>da(o,r))),biases:Array.from({length:t},()=>da(o,r)),activation:`sigmoid`}),c=t}let l=n===1?a:a/Math.sqrt(Math.max(1,c));return s.push({name:`output`,weights:Array.from({length:c},()=>Array.from({length:r},()=>da(o,l))),biases:Array.from({length:r},()=>da(o,l)),activation:`sigmoid`}),{layers:s}}function Ta(e,t){let n=la(`inputs`,e);if(t.layers.length===0)throw Error(`layered network must have at least one layer`);let r=[],i=[],a=e,o=n.cols;for(let[e,n]of t.layers.entries()){Ca(n,o,e);let t=fa(new xt(a).dot(new xt(n.weights)).data,n.biases),s=va(t,n.activation);r.push(t),i.push(s),a=s,o=n.biases.length}return{rawByLayer:r,activationsByLayer:i,predictions:i[i.length-1]}}function Ea(e,t,n,r){let i=la(`inputs`,e),a=la(`targets`,t);if(i.rows!==a.rows)throw Error(`inputs and targets must have the same number of rows`);let o=Ta(e,n);if(la(`predictions`,o.predictions).cols!==a.cols)throw Error(`prediction width must match target width`);let s=new xt(o.predictions).subtract(new xt(t)).data,c=2/(a.rows*a.cols),l=s.map(e=>e.map(e=>c*e)),u=Array(n.layers.length),d=n.layers.length-1;u[d]=ba(l,ya(o.rawByLayer[d],o.activationsByLayer[d],n.layers[d].activation));for(let e=d-1;e>=0;--e){let t=new xt(u[e+1]).dot(new xt(n.layers[e+1].weights).transpose()).data;u[e]=ba(t,ya(o.rawByLayer[e],o.activationsByLayer[e],n.layers[e].activation))}let f=[],p=[],m=n.layers.map((t,n)=>{let i=n===0?e:o.activationsByLayer[n-1],a=u[n],s=new xt(i).transpose().dot(new xt(a)).data,c=pa(a);return f.push(s),p.push(c),{...ca(t),weights:xa(t.weights,s,r),biases:Sa(t.biases,c,r)}});return{...o,errors:s,deltas:u,weightGradients:f,biasGradients:p,nextParameters:{layers:m},loss:ma(s)}}function Da(e,t,n=0,r){let i=Ta(e,t);if(n<0||n>=e.length)throw Error(`exampleIndex must refer to an input row`);let a=e[n],o=i.predictions[n];if(r!==void 0&&r.length!==o.length)throw Error(`target width must match prediction width`);let s,c,l;r!==void 0&&(c=o.map((e,t)=>e-r[t]),l=ha(c),s=Ea([a],[r],t,0).deltas);let u=t.layers.map((e,r)=>{let o=r===0?a:i.activationsByLayer[r-1][n],c=r===0?`input`:t.layers[r-1].name,l=i.rawByLayer[r][n],u=i.activationsByLayer[r][n].map((t,n)=>({neuron:`${e.name}[${n}]`,incoming:o.map((t,r)=>{let i=e.weights[r][n];return{source:`${c}[${r}]`,value:t,weight:i,contribution:t*i}}),bias:e.biases[n],rawSum:l[n],activation:e.activation,output:t,delta:s?.[r]?.[0]?.[n]}));return{layer:e.name,neurons:u}});return{exampleIndex:n,inputs:[...a],target:r===void 0?void 0:[...r],prediction:[...o],error:c,loss:l,layers:u}}function Oa(e){return Math.max(0,e.layers.length-1)}function ka(e){return e===void 0||e.length===0?`0x0`:`${e.length}x${e[0]?.length??0}`}function Aa(e,t){let n=ct(Qe({name:`ml-learning-linear-visualizer`,inputNames:[`x`],layers:[{name:`output`,weights:[[t.weight]],biases:[t.bias],activation:`none`,outputNames:[`prediction`]}]})),r=wt(n);return{predictions:Et(r,{x:e}).outputs.prediction??[],bytecodeInstructionCount:n.functions[0]?.instructions.length??0,matrixInstructionCount:r.instructions.length}}function ja(e,t,n={}){let r=Fa(e,t,n),i=Et(r.matrixPlan,r.matrixInputs);return{predictions:Ia(e,r.outputNames,i.outputs),bytecodeInstructionCount:r.bytecodeInstructionCount,matrixInstructionCount:r.matrixInstructionCount}}async function Ma(e,t,n={}){let r=Fa(e,t,n),i=await La();if(i.backend!==null){let t=await Tt(r.matrixPlan,r.matrixInputs,i.backend);return{predictions:Ia(e,r.outputNames,t.outputs),bytecodeInstructionCount:r.bytecodeInstructionCount,matrixInstructionCount:r.matrixInstructionCount,backend:`webgpu`}}let a=Et(r.matrixPlan,r.matrixInputs);return{predictions:Ia(e,r.outputNames,a.outputs),bytecodeInstructionCount:r.bytecodeInstructionCount,matrixInstructionCount:r.matrixInstructionCount,backend:`cpu`,fallbackReason:i.reason}}function Na(){return Bt.isNavigatorAvailable()}var Pa;function Fa(e,t,n){let r=t.layers[0],i=t.layers[t.layers.length-1];if(r===void 0||i===void 0)throw Error(`layered VM prediction requires at least one layer`);let a=e[0]?.length??r.weights.length,o=i.biases.length,s=n.inputNames??Array.from({length:a},(e,t)=>`input${t}`),c=n.outputNames??Array.from({length:o},(e,t)=>o===1?`prediction`:`output${t}`),l=ct(Qe({name:`ml-learning-layered-visualizer`,inputNames:s,layers:t.layers.map((e,n)=>({name:e.name,weights:e.weights,biases:e.biases,activation:za(e.activation),outputNames:n===t.layers.length-1?c:void 0}))})),u=wt(l);return{matrixPlan:u,matrixInputs:Object.fromEntries(s.map((t,n)=>[t,e.map(e=>e[n]??0)])),outputNames:c,bytecodeInstructionCount:l.functions[0]?.instructions.length??0,matrixInstructionCount:u.instructions.length}}function Ia(e,t,n){return e.map((e,r)=>t.map(e=>n[e]?.[r]??0))}async function La(){return Pa??=Ra(),Pa}async function Ra(){if(!Bt.isNavigatorAvailable())return{backend:null,reason:`WebGPU is not exposed by this browser`};try{let e=await Bt.createFromNavigator({powerPreference:`high-performance`});return{backend:e,reason:e===null?`WebGPU is not exposed by this browser`:void 0}}catch(e){return{backend:null,reason:e instanceof Error?e.message:`WebGPU initialization failed`}}}function za(e){return e===`linear`?`none`:e}function Ba(e){return{...e,defaultHiddenLayerCount:e.defaultHiddenLayerCount??1,hiddenLayerMin:e.hiddenLayerMin??1,hiddenLayerMax:e.hiddenLayerMax??4,learningRateMin:e.defaultLearningRate/20,learningRateMax:e.defaultLearningRate*8,learningRateStep:e.defaultLearningRate/20}}function Va(e){return e.rows.map(e=>[e.target])}function Ha(e){return e.rows.map(e=>e.input)}function Ua(e,t=e.defaultHiddenLayerCount){let n=Math.max(e.hiddenLayerMin,Math.min(e.hiddenLayerMax,Math.round(t)));return{epoch:0,hiddenLayerCount:n,parameters:wa(e.inputLabels.length,e.hiddenCount,n,1,e.seed,e.initialScale)}}function Wa(e,t){return ja(Ha(e),t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}).predictions.map(e=>e[0])}function Ga(e,t){let n=Wa(e,t);return n.reduce((t,n,r)=>{let i=n-e.rows[r].target;return t+i*i},0)/n.length}function Ka(e,t){let n=Wa(e,t);return n.reduce((t,n,r)=>t+Math.abs(n-e.rows[r].target),0)/n.length}function qa(e,t){return{epoch:t.epoch,loss:Ga(e,t),mae:Ka(e,t)}}function Ja(e,t,n){let r=Ea(Ha(e),Va(e),t.parameters,n),i={epoch:t.epoch+1,hiddenLayerCount:Oa(r.nextParameters),parameters:r.nextParameters};return{previousState:t,state:i,step:r,loss:Ga(e,i),mae:Ka(e,i)}}function Ya(e,t,n,r){let i=[],a=t;for(let t=0;t<r;t+=1){let t=Ja(e,a,n);i.push(t),a=t.state}return i}function Xa(e,t,n){return{...Da([e.rows[n].input],t.parameters,0,[e.rows[n].target]),exampleIndex:n}}var Za=[];for(let e of[-1,-.5,0,.5,1])for(let t of[-1,-.5,0,.5,1])Za.push({input:[e,t],target:+(e*e+t*t<=.55),label:`(${e}, ${t})`,group:e*e+t*t<=.55?`inside`:`outside`});var Qa=[];for(let e=0;e<12;e+=1){let t=Math.PI*e/11;Qa.push({input:[Math.cos(t),Math.sin(t)],target:0,label:`upper ${e+1}`,group:`upper`}),Qa.push({input:[1-Math.cos(t),.5-Math.sin(t)],target:1,label:`lower ${e+1}`,group:`lower`})}var $a=[Ba({id:`xnor`,title:`XNOR Gate`,category:`Logic`,summary:`Outputs 1 when the two inputs match and 0 when they differ.`,lesson:`The hidden layer learns two useful regions: both inputs off and both inputs on. The output neuron combines those regions into one decision.`,inputLabels:[`A`,`B`],outputLabel:`same?`,rows:[{input:[0,0],target:1,label:`A=0, B=0`,group:`same`},{input:[0,1],target:0,label:`A=0, B=1`,group:`different`},{input:[1,0],target:0,label:`A=1, B=0`,group:`different`},{input:[1,1],target:1,label:`A=1, B=1`,group:`same`}],hiddenCount:3,initialScale:2,seed:31,defaultLearningRate:1.4,chartKind:`surface`}),Ba({id:`absolute-value`,title:`Absolute Value`,category:`Regression`,summary:`Learns the V-shaped relationship y = |x| on normalized inputs.`,lesson:`A single line cannot bend at zero. Hidden neurons can split the input range into left and right regions, then recombine them into a V.`,inputLabels:[`x`],outputLabel:`|x|`,rows:[-1,-.75,-.5,-.25,0,.25,.5,.75,1].map(e=>({input:[e],target:Math.abs(e),label:`x=${e}`})),hiddenCount:6,initialScale:3,seed:12,defaultLearningRate:1.8,chartKind:`curve`}),Ba({id:`piecewise-pricing`,title:`Piecewise Pricing`,category:`Regression`,summary:`Approximates a stepped shipping-price schedule from package weight.`,lesson:`Hidden neurons can behave like soft thresholds. Several thresholds together make a stair-step curve.`,inputLabels:[`weight`],outputLabel:`price tier`,rows:[[.05,.12],[.15,.12],[.25,.25],[.35,.25],[.45,.55],[.55,.55],[.7,.88],[.85,.88],[1,.88]].map(([e,t])=>({input:[e],target:t,label:`${Math.round(e*40)} lb`})),hiddenCount:6,initialScale:3,seed:19,defaultLearningRate:2,chartKind:`curve`}),Ba({id:`circle-classifier`,title:`Circle Classifier`,category:`Classification`,summary:`Classifies whether a point is inside a circle.`,lesson:`The hidden layer combines several soft boundaries. Together they can carve out a round-ish region even though each neuron is simple.`,inputLabels:[`x`,`y`],outputLabel:`inside?`,rows:Za,hiddenCount:8,initialScale:3,seed:37,defaultLearningRate:2.2,chartKind:`surface`}),Ba({id:`two-moons`,title:`Two Moons`,category:`Classification`,summary:`Separates two curved bands that no single straight boundary can split.`,lesson:`The hidden layer remaps curved geometry into features the output neuron can combine into a useful decision.`,inputLabels:[`x`,`y`],outputLabel:`moon`,rows:Qa,hiddenCount:10,initialScale:3,seed:43,defaultLearningRate:1.8,chartKind:`surface`}),Ba({id:`interaction-features`,title:`Interaction Features`,category:`Tabular`,summary:`Predicts a normalized house-value score from bedrooms, bathrooms, and garage.`,lesson:`The hidden layer can learn combinations, like garage plus enough rooms, instead of treating each input as a separate straight-line effect.`,inputLabels:[`bedrooms`,`bathrooms`,`garage`],outputLabel:`value score`,rows:[{input:[.2,.25,0],target:.08,label:`1 bed, 1 bath, no garage`},{input:[.4,.25,0],target:.18,label:`2 bed, 1 bath, no garage`},{input:[.4,.5,0],target:.32,label:`2 bed, 2 bath, no garage`},{input:[.6,.5,0],target:.45,label:`3 bed, 2 bath, no garage`},{input:[.6,.5,1],target:.72,label:`3 bed, 2 bath, garage`},{input:[.8,.5,0],target:.58,label:`4 bed, 2 bath, no garage`},{input:[.8,.75,1],target:.9,label:`4 bed, 3 bath, garage`},{input:[1,.75,1],target:.96,label:`5 bed, 3 bath, garage`},{input:[1,1,0],target:.76,label:`5 bed, 4 bath, no garage`},{input:[.2,.5,1],target:.35,label:`1 bed, 2 bath, garage`}],hiddenCount:7,initialScale:3,seed:51,defaultLearningRate:1.8,chartKind:`table`})],eo=class extends Error{kind;constructor(e){super(`No handler registered for instruction kind: '${e}'`),this.name=`UnknownInstructionError`,this.kind=e}},to=class extends Error{kind;constructor(e){super(`Handler already registered for instruction kind: '${e}'`),this.name=`DuplicateHandlerError`,this.kind=e}},no=class extends Error{constructor(e){super(`export() is not supported by the ${e} backend. Use a backend that supports pixel readback (Canvas, Metal, Cairo).`),this.name=`ExportNotSupportedError`}},ro=class extends Error{constructor(){super(`execute() and patch() require a non-null context`),this.name=`NullContextError`}},io=class{table=new Map;clearFn;exportFn;constructor(e,t){this.clearFn=e,this.exportFn=t}register(e,t){if(this.table.has(e))throw new to(e);this.table.set(e,t)}dispatch(e,t){let n=this.table.get(e.kind)??this.table.get(`*`);if(!n)throw new eo(e.kind);n(e,t,this)}execute(e,t){if(t==null)throw new ro;this.clearFn(t,e.background,e.width,e.height);for(let n of e.instructions)this.dispatch(n,t)}patch(e,t,n,r){if(n==null)throw new ro;if(!r){this.execute(t,n);return}let{onDelete:i,onInsert:a,onUpdate:o}=r,s=new Map,c=new Map;for(let t of e.instructions)t.id&&s.set(t.id,t);for(let e of t.instructions)e.id&&c.set(e.id,e);for(let[e,t]of s)c.has(e)||i?.(t);for(let n=0;n<t.instructions.length;n++){let r=t.instructions[n];if(r.id&&s.has(r.id)){let e=s.get(r.id);ao(r,e)||o?.(e,r)}else if(n<e.instructions.length){let t=e.instructions[n];ao(r,t)||o?.(t,r)}else a?.(r,n)}}export(e,t){if(!this.exportFn)throw new no(`this`);let n={scale:t?.scale??1,channels:t?.channels??4,bit_depth:t?.bit_depth??8,color_space:t?.color_space??`srgb`};return this.exportFn(e,this,n)}registeredKinds(){return[...this.table.keys()]}};function ao(e,t){if(e===t)return!0;if(e==null||t==null||typeof e!=typeof t)return!1;if(typeof e!=`object`)return e===t;if(Array.isArray(e)!==Array.isArray(t))return!1;if(Array.isArray(e)&&Array.isArray(t)){if(e.length!==t.length)return!1;for(let n=0;n<e.length;n++)if(!ao(e[n],t[n]))return!1;return!0}let n=e,r=t,i=Object.keys(n),a=Object.keys(r);if(i.length!==a.length)return!1;for(let e of i)if(!Object.prototype.hasOwnProperty.call(r,e)||!ao(n[e],r[e]))return!1;return!0}function oo(){return{defs:[],elements:[],clipCounter:0,filterCounter:0}}function so(e){return e.replace(/&/g,`&amp;`).replace(/"/g,`&quot;`).replace(/</g,`&lt;`).replace(/>/g,`&gt;`)}function co(e){return e.replace(/&/g,`&amp;`).replace(/</g,`&lt;`).replace(/>/g,`&gt;`)}function lo(e){let t=[];return t.push(`fill="${so(e.fill??`none`)}"`),e.stroke&&(t.push(`stroke="${so(e.stroke)}"`),t.push(`stroke-width="${N(e.stroke_width??1,`stroke_width`)}"`)),e.opacity!==void 0&&e.opacity!==1&&t.push(`opacity="${N(e.opacity,`opacity`)}"`),t.join(` `)}function uo(e){return e?` id="${so(e)}"`:``}function fo(e){let t=e=>+e.toFixed(4);return e.map(e=>{switch(e.kind){case`move_to`:return`M ${t(e.x)} ${t(e.y)}`;case`line_to`:return`L ${t(e.x)} ${t(e.y)}`;case`quad_to`:return`Q ${t(e.cx)} ${t(e.cy)} ${t(e.x)} ${t(e.y)}`;case`cubic_to`:return`C ${t(e.cx1)} ${t(e.cy1)} ${t(e.cx2)} ${t(e.cy2)} ${t(e.x)} ${t(e.y)}`;case`arc_to`:return`A ${t(e.rx)} ${t(e.ry)} ${t(e.x_rotation)} ${+!!e.large_arc} ${+!!e.sweep} ${t(e.x)} ${t(e.y)}`;case`close`:return`Z`}}).join(` `)}function po(e){if(!e)return``;let[t,n,r,i,a,o]=e;return` transform="matrix(${N(t,`transform.a`)},${N(n,`transform.b`)},${N(r,`transform.c`)},${N(i,`transform.d`)},${N(a,`transform.e`)},${N(o,`transform.f`)})"`}function N(e,t){if(!Number.isFinite(e))throw RangeError(`PaintVM SVG: ${t} must be a finite number, got ${e}`);return String(e)}function mo(e,t){if(!t||t.length===0)return``;let n=[],r=`SourceGraphic`;for(let e=0;e<t.length;e++){let i=t[e],a=`f${e}`;switch(i.kind){case`blur`:n.push(`<feGaussianBlur in="${r}" stdDeviation="${N(i.radius,`blur.radius`)}" result="${a}"/>`);break;case`drop_shadow`:n.push(`<feDropShadow dx="${N(i.dx,`drop_shadow.dx`)}" dy="${N(i.dy,`drop_shadow.dy`)}" stdDeviation="${N(i.blur,`drop_shadow.blur`)}" flood-color="${so(i.color)}" result="${a}"/>`);break;case`color_matrix`:{let e=i.matrix.map((e,t)=>N(e,`color_matrix.matrix[${t}]`));n.push(`<feColorMatrix in="${r}" type="matrix" values="${e.join(` `)}" result="${a}"/>`);break}case`brightness`:{let e=N(i.amount,`brightness.amount`);n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncR type="linear" slope="${e}"/><feFuncG type="linear" slope="${e}"/><feFuncB type="linear" slope="${e}"/></feComponentTransfer>`)}break;case`contrast`:{let e=i.amount,t=-(i.amount-1)/2;n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncR type="linear" slope="${N(e,`contrast.amount`)}" intercept="${N(t,`contrast.intercept`)}"/><feFuncG type="linear" slope="${N(e,`contrast.amount`)}" intercept="${N(t,`contrast.intercept`)}"/><feFuncB type="linear" slope="${N(e,`contrast.amount`)}" intercept="${N(t,`contrast.intercept`)}"/></feComponentTransfer>`)}break;case`saturate`:n.push(`<feColorMatrix in="${r}" type="saturate" values="${N(i.amount,`saturate.amount`)}" result="${a}"/>`);break;case`hue_rotate`:n.push(`<feColorMatrix in="${r}" type="hueRotate" values="${N(i.angle,`hue_rotate.angle`)}" result="${a}"/>`);break;case`invert`:{let e=N(i.amount,`invert.amount`),t=N(-i.amount,`invert.neg_amount`);n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncR type="linear" slope="${t}" intercept="${e}"/><feFuncG type="linear" slope="${t}" intercept="${e}"/><feFuncB type="linear" slope="${t}" intercept="${e}"/></feComponentTransfer>`)}break;case`opacity`:n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncA type="linear" slope="${N(i.amount,`opacity.amount`)}"/></feComponentTransfer>`);break}r=a}return`<filter id="${so(e)}">${n.join(``)}</filter>`}var ho=new Set([`normal`,`multiply`,`screen`,`overlay`,`darken`,`lighten`,`color-dodge`,`color-burn`,`hard-light`,`soft-light`,`difference`,`exclusion`,`hue`,`saturation`,`color`,`luminosity`]);function go(e){let t=e.replace(/_/g,`-`);return ho.has(t)?t:`normal`}function _o(e,t){let n=lo(e),r=e.corner_radius===void 0?``:` rx="${N(e.corner_radius,`rect.corner_radius`)}"`;t.elements.push(`<rect${uo(e.id)} x="${N(e.x,`rect.x`)}" y="${N(e.y,`rect.y`)}" width="${N(e.width,`rect.width`)}" height="${N(e.height,`rect.height`)}"${r} ${n}/>`)}function vo(e,t){let n=lo(e);t.elements.push(`<ellipse${uo(e.id)} cx="${N(e.cx,`ellipse.cx`)}" cy="${N(e.cy,`ellipse.cy`)}" rx="${N(e.rx,`ellipse.rx`)}" ry="${N(e.ry,`ellipse.ry`)}" ${n}/>`)}var yo=new Set([`nonzero`,`evenodd`]),P=new Set([`butt`,`round`,`square`]),bo=new Set([`miter`,`round`,`bevel`]);function xo(e,t){let n=fo(e.commands),r=e.fill_rule&&yo.has(e.fill_rule)?e.fill_rule:`nonzero`,i=r===`nonzero`?``:` fill-rule="${r}"`,a=e.stroke_cap&&P.has(e.stroke_cap)?` stroke-linecap="${e.stroke_cap}"`:``,o=e.stroke_join&&bo.has(e.stroke_join)?` stroke-linejoin="${e.stroke_join}"`:``,s=lo(e);t.elements.push(`<path${uo(e.id)} d="${so(n)}"${i}${a}${o} ${s}/>`)}function So(e,t){let n=e.fill??`#000000`,r=e.glyphs.map(e=>{let t=e.glyph_id,n=Number.isInteger(t)&&t>=0&&t<=1114111?t:65533;return`<tspan x="${N(e.x,`glyph.x`)}" y="${N(e.y,`glyph.y`)}">&#${n};</tspan>`});t.elements.push(`<text${uo(e.id)} font-size="${N(e.font_size,`glyph_run.font_size`)}" fill="${so(n)}">${r.join(``)}</text>`)}function Co(e){let t;if(e.startsWith(`canvas:`))t=e.slice(7);else if(e.startsWith(`svg:`))t=e.slice(4);else return{family:`sans-serif`,weight:`400`,style:``};let n=t.indexOf(`@`),r=n>=0?t.slice(0,n):t,i=(n>=0?t.slice(n+1):``).split(`:`),a=i[1],o=i[2],s=r.replace(/[^a-zA-Z0-9 ,\-_.]/g,``)||`sans-serif`,c=`400`;if(a!==void 0){let e=Number(a);Number.isFinite(e)&&e>=1&&e<=1e3&&(c=String(Math.round(e)))}let l=o!==void 0&&new Set([`italic`,`oblique`]).has(o)?o:``;return{family:s,weight:c,style:l}}function wo(e){switch(e){case`center`:return`middle`;case`end`:return`end`;default:return`start`}}function To(e,t){if(!Number.isFinite(e.font_size))throw RangeError(`PaintVM SVG: font_size must be a finite number, got ${e.font_size}`);let{family:n,weight:r,style:i}=Co(e.font_ref),a=[`font-family="${so(n)}"`,`font-size="${N(e.font_size,`text.font_size`)}"`];r!==`400`&&r!==`normal`&&a.push(`font-weight="${so(r)}"`),i&&a.push(`font-style="${so(i)}"`);let o=wo(e.text_align);o!==`start`&&a.push(`text-anchor="${o}"`),t.elements.push(`<text${uo(e.id)} x="${N(e.x,`text.x`)}" y="${N(e.y,`text.y`)}" ${a.join(` `)} fill="${so(e.fill)}">${co(e.text)}</text>`)}function Eo(e,t,n){let r=po(e.transform),i=e.opacity!==void 0&&e.opacity!==1?` opacity="${N(e.opacity,`group.opacity`)}"`:``;t.elements.push(`<g${uo(e.id)}${r}${i}>`);for(let r of e.children)n.dispatch(r,t);t.elements.push(`</g>`)}function Do(e,t,n){let r=e.id?`filter-${e.id}`:`filter-${t.filterCounter++}`,i=mo(r,e.filters);i&&t.defs.push(i);let a=i?` filter="url(#${so(r)})"`:``,o=e.blend_mode&&e.blend_mode!==`normal`?` style="mix-blend-mode:${go(e.blend_mode)}"`:``,s=po(e.transform),c=e.opacity!==void 0&&e.opacity!==1?` opacity="${N(e.opacity,`layer.opacity`)}"`:``;t.elements.push(`<g${uo(e.id)}${s}${c}${a}${o}>`);for(let r of e.children)n.dispatch(r,t);t.elements.push(`</g>`)}function Oo(e,t){let n=e.stroke_cap&&P.has(e.stroke_cap)?` stroke-linecap="${e.stroke_cap}"`:``,r=N(e.stroke_width??1,`line.stroke_width`);t.elements.push(`<line${uo(e.id)} x1="${N(e.x1,`line.x1`)}" y1="${N(e.y1,`line.y1`)}" x2="${N(e.x2,`line.x2`)}" y2="${N(e.y2,`line.y2`)}" stroke="${so(e.stroke)}" stroke-width="${r}"${n} fill="none"/>`)}function ko(e,t,n){let r=e.id?`clip-${e.id}`:`clip-${t.clipCounter++}`;t.defs.push(`<clipPath id="${so(r)}"><rect x="${N(e.x,`clip.x`)}" y="${N(e.y,`clip.y`)}" width="${N(e.width,`clip.width`)}" height="${N(e.height,`clip.height`)}"/></clipPath>`),t.elements.push(`<g clip-path="url(#${so(r)})">`);for(let r of e.children)n.dispatch(r,t);t.elements.push(`</g>`)}function Ao(e,t){if(!e.id)return;let n=e.stops.map((e,t)=>`<stop offset="${N(e.offset,`gradient.stops[${t}].offset`)}" stop-color="${so(e.color)}"/>`).join(``),r;r=e.gradient_kind===`linear`?`<linearGradient id="${so(e.id)}" x1="${N(e.x1??0,`gradient.x1`)}" y1="${N(e.y1??0,`gradient.y1`)}" x2="${N(e.x2??0,`gradient.x2`)}" y2="${N(e.y2??0,`gradient.y2`)}" gradientUnits="userSpaceOnUse">`+n+`</linearGradient>`:`<radialGradient id="${so(e.id)}" cx="${N(e.cx??0,`gradient.cx`)}" cy="${N(e.cy??0,`gradient.cy`)}" r="${N(e.r??0,`gradient.r`)}" gradientUnits="userSpaceOnUse">`+n+`</radialGradient>`,t.defs.push(r)}function jo(e){let t=e.replace(/\0/g,``),n=t.toLowerCase().trimStart();return n.startsWith(`data:`)||n.startsWith(`https:`)?t:`data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==`}function Mo(e,t){let n;n=typeof e.src==`string`?jo(e.src):`data:image/png;base64,`;let r=e.opacity!==void 0&&e.opacity!==1?` opacity="${N(e.opacity,`image.opacity`)}"`:``;t.elements.push(`<image${uo(e.id)} x="${N(e.x,`image.x`)}" y="${N(e.y,`image.y`)}" width="${N(e.width,`image.width`)}" height="${N(e.height,`image.height`)}" href="${so(n)}"${r}/>`)}function No(){let e=new io((e,t)=>{e.defs.length=0,e.elements.length=0,e.clipCounter=0,e.filterCounter=0},()=>{throw new no(`SVG`)});return e.register(`rect`,(e,t)=>{e.kind===`rect`&&_o(e,t)}),e.register(`ellipse`,(e,t)=>{e.kind===`ellipse`&&vo(e,t)}),e.register(`path`,(e,t)=>{e.kind===`path`&&xo(e,t)}),e.register(`glyph_run`,(e,t)=>{e.kind===`glyph_run`&&So(e,t)}),e.register(`text`,(e,t)=>{e.kind===`text`&&To(e,t)}),e.register(`group`,(e,t,n)=>{e.kind===`group`&&Eo(e,t,n)}),e.register(`layer`,(e,t,n)=>{e.kind===`layer`&&Do(e,t,n)}),e.register(`line`,(e,t)=>{e.kind===`line`&&Oo(e,t)}),e.register(`clip`,(e,t,n)=>{e.kind===`clip`&&ko(e,t,n)}),e.register(`gradient`,(e,t)=>{e.kind===`gradient`&&Ao(e,t)}),e.register(`image`,(e,t)=>{e.kind===`image`&&Mo(e,t)}),e}function Po(e){let t=No(),n=oo();return t.execute(e,n),Fo(e,n)}function Fo(e,t){let n=N(e.width,`scene.width`),r=N(e.height,`scene.height`),i=[];return i.push(`<svg xmlns="http://www.w3.org/2000/svg" width="${n}" height="${r}">`),t.defs.length>0&&i.push(`<defs>${t.defs.join(``)}</defs>`),e.background!==`transparent`&&e.background!==`none`&&i.push(`<rect width="${n}" height="${r}" fill="${so(e.background)}"/>`),i.push(...t.elements),i.push(`</svg>`),i.join(``)}function Io(e,t,n,r,i){return{width:e,height:t,background:n,instructions:r,...i}}function Lo(e,t,n,r,i){return{kind:`rect`,x:e,y:t,width:n,height:r,...i}}function Ro(e,t,n,r,i){return{kind:`ellipse`,cx:e,cy:t,rx:n,ry:r,...i}}function zo(e,t,n,r,i,a){return{kind:`line`,x1:e,y1:t,x2:n,y2:r,stroke:i,...a}}function Bo(e,t,n,r,i,a,o){return{kind:`text`,x:e,y:t,text:n,font_ref:r,font_size:i,fill:a,...o}}var Vo=`svg:ui-sans-serif@12`,Ho=`#5d6d68`,Uo=`#ffffff`,Wo=`rgba(23, 32, 28, 0.16)`,Go=`#237a57`,Ko=`#2563eb`,qo=`#c2413b`,Jo=`#b7791f`,Yo=`#6d5bd0`,Xo=[`#2563eb`,`#237a57`,`#b7791f`,`#6d5bd0`,`#c2413b`,`#0f766e`,`#be185d`,`#7c3aed`,`#ca8a04`,`#0284c7`];function Zo({model:e,lastStep:t,learningRate:n,lossKind:r,samplePoint:i,pointCount:a}){return(0,T.jsx)(ts,{title:`Learning flow`,summary:`Forward pass and gradient descent`,svg:$o(e,t,n,r,i,a)})}function Qo({example:e,state:t,selectedRow:n,selectedIndex:r,prediction:i,lastStep:a,learningRate:o}){return(0,T.jsx)(ts,{title:`Neural graph`,summary:`Hidden layer learning flow`,svg:es(e,t,n,r,i,a,o)})}function $o(e,t,n,r=`mse`,i={x:0,y:0},a=1){return Po(ns(e,t,n,r,i,a))}function es(e,t,n,r,i,a,o){return Po(rs(e,t,n,r,i,a,o))}function ts({title:e,summary:t,svg:n}){return(0,T.jsxs)(`section`,{className:`network-panel`,"aria-label":t,children:[(0,T.jsxs)(`div`,{className:`history__topline`,children:[(0,T.jsx)(`span`,{children:e}),(0,T.jsx)(`strong`,{children:t})]}),(0,T.jsx)(`div`,{className:`network-svg`,dangerouslySetInnerHTML:{__html:n}})]})}function ns(e,t,n,r,i,a){let o=t?.previousState??e,s=i.x*o.weight+o.bias,c=s-i.y,l=r===`mse`?c*c:Math.abs(c),u={id:`input`,label:`x`,value:F(i.x),x:100,y:150,tone:`input`},d={id:`bias`,label:`bias`,value:F(o.bias),x:100,y:232,tone:`bias`},f={id:`sum`,label:`sum`,value:`x*w+b`,x:318,y:150,tone:`hidden`},p={id:`output`,label:`pred`,value:F(s),x:540,y:150,tone:`output`},m={id:`target`,label:`target`,value:F(i.y),x:540,y:232,tone:`bias`},h={id:`loss`,label:r,value:F(l),x:760,y:190,tone:`output`},g=t===null?0:-n*t.gradientWeight,_=t===null?0:-n*t.gradientBias,v=t===null?`waiting for first step`:`dL/dw ${F(t.gradientWeight)}  dL/db ${F(t.gradientBias)}`,y=t===null?`run Step to update weights`:`w ${F(t.previousState.weight)} -> ${F(e.weight)}`,b=t===null?`lr ${F(n)}`:`b ${F(t.previousState.bias)} -> ${F(e.bias)}`;return Io(920,650,`#ffffff`,[Lo(16,16,888,618,{fill:Uo,stroke:Wo,stroke_width:1,corner_radius:8}),...is(`1 forward pass`,36,48),...cs(u,f,o.weight,`w ${F(o.weight)}`),...cs(d,f,o.bias,`b ${F(o.bias)}`),...cs(f,p,1,`linear`),...cs(p,h,c,`err ${_s(c)}`,.56),...cs(m,h,-1,`truth`,.56),...ds(u),...ds(d),...ds(f),...ds(p),...ds(m),...ds(h),...is(`2 folded training loop`,36,292),...as(36,322,152,`input batch`,[`${a} rows`,`sample x ${F(i.x)}`,`target ${F(i.y)}`],Ko),...os(194,368,242,368,Ko,`feed`),...as(252,322,158,`prediction`,[`yhat=x*w+b`,`yhat ${F(s)}`,`activation linear`],Go),...os(416,368,464,368,Ko,`compare`),...as(474,322,162,`error + loss`,[`error ${_s(c)}`,`${r.toUpperCase()} ${F(l)}`,`batch loss ${F(t?.previousLoss??l)}`],qo),...os(555,448,555,470,qo,``),...as(474,470,184,`gradient descent`,[v,`dw step ${_s(g)}`,`db step ${_s(_)}`],Yo),...os(464,516,416,516,Yo,`apply lr`),...as(252,470,158,`parameter update`,[y,b,`next run uses them`],Yo),...os(242,516,194,516,Yo,`store`),...as(36,470,152,`model state`,[`w ${F(e.weight)}`,`b ${F(e.bias)}`,`epoch ${e.epoch}`],Ko),...fs(`parameter update: new = old - learningRate * gradient`,474,626,13,Ho),...fs(`epoch ${e.epoch}`,36,72,13,Ho),...fs(`line width follows |weight|; green is positive, red is negative`,476,72,12,Ho)],{id:`linear-neural-network-diagram`})}function rs(e,t,n,r,i,a,o){let s=n.input,c=i-n.target,l=Ta([s],t.parameters),u=t.parameters.layers.slice(0,-1),d=t.parameters.layers[t.parameters.layers.length-1],f=Math.max(e.inputLabels.length,d.biases.length,...u.map(e=>e.biases.length)),p=Math.max(318,106+Math.max(1,f-1)*66),m=p+92,h=m+344,g=290+Math.max(0,u.length-1)*190+210,_=g+150,v=Math.max(1080,_+98),y=ms(e.inputLabels.length,106,p),b=u.map((e,t)=>290+t*190),x=106+(p-106)*.42,ee=x+86,S=x+42,C=e.inputLabels.map((e,t)=>({id:`input-${t}`,label:e,value:F(s[t]??0),x:96,y:y[t],tone:`input`})),te={id:`bias`,label:`bias`,value:`1`,x:96,y:p+56,tone:`bias`},ne=u.map((e,t)=>{let n=ms(e.biases.length,106,p);return e.biases.map((e,r)=>({id:`hidden-${t}-${r}`,label:`h${t+1}.${r+1}`,value:F(l.activationsByLayer[t][0][r]??0),x:b[t],y:n[r],tone:`hidden`}))}),w=ne[ne.length-1]??[],re={id:`output`,label:e.outputLabel,value:F(i),x:g,y:x,tone:`output`},ie={id:`target`,label:`target`,value:F(n.target),x:g,y:ee,tone:`bias`},ae={id:`loss`,label:`mse`,value:F(c*c),x:_,y:S,tone:`output`},T=[Lo(16,16,v-32,h-32,{fill:Uo,stroke:Wo,stroke_width:1,corner_radius:8}),...is(`1 selected forward pass`,32,48),...fs(`epoch ${t.epoch}`,32,70,13,Ho),...fs(`edge color follows source node; line width follows |weight|`,v-412,48,13,Ho)];for(let[e,t]of ne.entries()){let n=u[e],r=e===0?C:ne[e-1];for(let[i,a]of r.entries())for(let[r,o]of t.entries()){let s=n.weights[i][r],c=e===0&&u.length<=2&&t.length<=8;T.push(...cs(a,o,s,c?F(s):``,.34,ls(a.id)))}}for(let[e,t]of ne.entries()){let n=u[e];for(let[r,i]of t.entries()){let a=n.biases[r],o=e===0&&u.length===1&&t.length<=8;T.push(...cs(te,i,a,o?F(a):``,.26,ls(`bias-${e}`)))}}for(let[e,t]of w.entries()){let n=d.weights[e][0],r=u.length<=2&&w.length<=8;T.push(...cs(t,re,n,r?F(n):``,.42,ls(t.id)))}T.push(...cs(te,re,d.biases[0]??0,u.length===1?F(d.biases[0]??0):``,.28,ls(`bias-output`)),...cs(re,ae,c,`err ${_s(c)}`,.62),...cs(ie,ae,-1,`truth`,.62),...C.flatMap(ds),...ds(te),...ne.flatMap(e=>e.flatMap(ds)),...ds(re),...ds(ie),...ds(ae));let oe=a===null?`input-hidden gradients waiting`:`dL/dW1 ${hs(a.step.weightGradients[0])}`,se=a?.step.weightGradients[a.step.weightGradients.length-1],ce=a?.step.biasGradients[a.step.biasGradients.length-1]?.[0]??0,le=a?.step.deltas[a.step.deltas.length-1]?.[r]?.[0]??0,E=a?.step.deltas.slice(0,-1).flatMap(e=>e[r]??[])??[],D=E.length===0?0:E.reduce((e,t)=>Math.max(e,Math.abs(t)),0),ue=a===null?`waiting for first step`:`max hidden delta ${F(D)}`,de=m+32,fe=m+180;return T.push(...is(`2 folded loss + update loop`,32,m),...as(32,de,158,`input row`,[n.label,`inputs ${s.map(e=>F(e)).join(`, `)}`,`target ${F(n.target)}`],Ko),...os(196,de+46,244,de+46,Ko,`forward`),...as(254,de,162,`prediction`,[`${t.hiddenLayerCount} x hidden[${e.hiddenCount}]`,`${e.outputLabel} ${F(i)}`,`error ${_s(c)}`],Go),...os(422,de+46,470,de+46,qo,`loss`),...as(480,de,158,`mse + deltas`,[`row mse ${F(c*c)}`,`output delta ${F(le)}`,ue],qo),...os(559,de+126,559,fe,qo,``),...as(480,fe,186,`gradient matrices`,[oe,`dL/dW${t.parameters.layers.length} ${hs(se)}`,`db out ${_s(-o*ce)}`],Yo),...os(470,fe+46,422,fe+46,Yo,`apply lr`),...as(254,fe,162,`parameter update`,[`lr ${F(o)}`,`${t.parameters.layers.length} weight matrices`,`next batch uses them`],Yo),...os(244,fe+46,196,fe+46,Yo,`store`),...as(32,fe,158,`model state`,[`epoch ${t.epoch}`,`${t.hiddenLayerCount} hidden layers`,`${e.hiddenCount} neurons/layer`],Ko),...fs(`new weights = old weights - learningRate * gradient`,32,h-24,13,Ho),...fs(`scroll the graph to inspect larger networks`,v-336,h-24,12,Ho)),Io(v,h,`#ffffff`,T,{id:`hidden-neural-network-diagram`})}function is(e,t,n){return[Lo(t-8,n-17,Math.max(126,e.length*8),24,{fill:`rgba(35, 122, 87, 0.1)`,stroke:`rgba(35, 122, 87, 0.18)`,stroke_width:1,corner_radius:6}),...fs(e,t,n,13,Go)]}function as(e,t,n,r,i,a){let o=[Lo(e,t,n,126,{fill:`#ffffff`,stroke:`rgba(23, 32, 28, 0.12)`,stroke_width:1,corner_radius:8}),Lo(e,t,n,30,{fill:`rgba(247, 248, 243, 0.95)`,stroke:`rgba(23, 32, 28, 0.08)`,stroke_width:1,corner_radius:8}),...fs(r,e+12,t+21,12,a)];for(let[r,a]of i.entries())o.push(...fs(vs(a,n),e+12,t+54+r*22,11,Ho));return o}function os(e,t,n,r,i,a){return ss(e,t,n,r,i,a,2)}function ss(e,t,n,r,i,a,o,s=.5,c=-7){let l=Math.atan2(r-t,n-e),u=l+Math.PI*.82,d=l-Math.PI*.82,f=e+(n-e)*s,p=t+(r-t)*s+c,m=[zo(e,t,n,r,i,{stroke_width:o,stroke_cap:`round`}),zo(n,r,n+Math.cos(u)*9,r+Math.sin(u)*9,i,{stroke_width:o,stroke_cap:`round`}),zo(n,r,n+Math.cos(d)*9,r+Math.sin(d)*9,i,{stroke_width:o,stroke_cap:`round`})];return a.length>0&&m.push(...ps(a,f,p,10,i)),m}function cs(e,t,n,r,i=.5,a){let o=e.x+(t.x-e.x)*i,s=e.y+(t.y-e.y)*i,c=a??(n>=0?Go:qo),l=Math.min(7,1.4+Math.abs(n)*.75),{x1:u,y1:d,x2:f,y2:p}=us(e.x,e.y,t.x,t.y,33,36),m=[...ss(u,d,f,p,c,``,l)];return r.length>0&&(m.push(Lo(o-28,s-14,56,20,{fill:`rgba(255, 255, 255, 0.86)`,stroke:`rgba(23, 32, 28, 0.08)`,stroke_width:1,corner_radius:5})),m.push(...ps(r,o,s+4,10,c))),m}function ls(e){let t=0;for(let n=0;n<e.length;n+=1)t=t*31+e.charCodeAt(n)>>>0;return Xo[t%Xo.length]}function us(e,t,n,r,i,a){let o=n-e,s=r-t,c=Math.hypot(o,s);if(c===0)return{x1:e,y1:t,x2:n,y2:r};let l=o/c,u=s/c;return{x1:e+l*i,y1:t+u*i,x2:n-l*a,y2:r-u*a}}function ds(e){let t=gs(e.tone);return[Ro(e.x,e.y,28,28,{fill:t,stroke:`#ffffff`,stroke_width:3}),Ro(e.x,e.y,31,31,{stroke:Wo,stroke_width:1}),...ps(e.label,e.x,e.y-3,11,`#ffffff`),...ps(e.value,e.x,e.y+12,10,`#ffffff`)]}function fs(e,t,n,r,i,a=`start`){return[Bo(t,n,e,Vo,r,i,{text_align:a})]}function ps(e,t,n,r,i){return fs(e,t,n,r,i,`center`)}function ms(e,t,n){if(e<=1)return[(t+n)/2];let r=n-t;return Array.from({length:e},(n,i)=>t+r*i/(e-1))}function hs(e){return e===void 0||e.length===0?`0x0`:`${e.length}x${e[0]?.length??0}`}function gs(e){switch(e){case`input`:return Ko;case`hidden`:return Go;case`output`:return Jo;case`bias`:return`#6d5bd0`}}function F(e){return Number.isFinite(e)?Math.abs(e)>=10?e.toFixed(1):e.toFixed(2):`0`}function _s(e){return`${e>=0?`+`:``}${F(e)}`}function vs(e,t){let n=Math.max(10,Math.floor(t/7.2));return e.length<=n?e:`${e.slice(0,n-3)}...`}var I={width:720,height:410,padLeft:58,padRight:24,padTop:24,padBottom:48,xMin:-1,xMax:1,yMin:-.08,yMax:1.08},ys=460;function bs(e,t=3){return Number.isFinite(e)?Math.abs(e)<.01&&e!==0?e.toExponential(2):e.toFixed(t):`0`}function xs(e,t,n){return Math.min(n,Math.max(t,e))}function Ss(e){return`${e} hidden layer${e===1?``:`s`}`}function Cs(e,t){let n=t.width-t.padLeft-t.padRight;return t.padLeft+(e-t.xMin)/(t.xMax-t.xMin)*n}function ws(e,t){let n=t.height-t.padTop-t.padBottom;return t.padTop+(1-(e-t.yMin)/(t.yMax-t.yMin))*n}function Ts(e){if(e.length===0)return``;let t=Math.max(...e.map(e=>e.loss),1e-6),n=e[0].epoch,r=Math.max(e[e.length-1].epoch-n,1);return e.map((e,i)=>{let a=(e.epoch-n)/r*250,o=74-xs(e.loss/t,0,1)*74;return`${i===0?`M`:`L`} ${a.toFixed(2)} ${o.toFixed(2)}`}).join(` `)}function Es(){return Array.from({length:121},(e,t)=>I.xMin+t/120*(I.xMax-I.xMin))}function Ds(e,t){return e.map((e,n)=>[e,t[n]??0]).map(([e,t],n)=>`${n===0?`M`:`L`} ${Cs(e,I).toFixed(2)} ${ws(t,I).toFixed(2)}`).join(` `)}function Os(e,t){let n=Es();return Ds(n,ja(n.map(e=>[e]),t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}).predictions.map(e=>e[0]??0))}function ks(e){let t=[],n=[],r=e.rows.map(e=>e.input[0]),i=e.rows.map(e=>e.input[1]),a=Math.min(...r,-1),o=Math.max(...r,1),s=Math.min(...i,-1),c=Math.max(...i,1),l=Math.max((o-a)*.08,.15),u=Math.max((c-s)*.08,.15);for(let e=0;e<26;e+=1)for(let r=0;r<26;r+=1){let i=a-l+r/25*(o-a+l*2),d=s-u+e/25*(c-s+u*2);n.push([i,d]),t.push({x:r,y:e,value:0})}return{cells:t,inputs:n}}function As(e,t){let n=ks(e),r=ja(n.inputs,t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}).predictions;return n.cells.map((e,t)=>({...e,value:r[t]?.[0]??0}))}function js(e,t){return{rowPredictions:Wa(e,t),curvePath:e.chartKind===`curve`?Os(e,t):``,surfaceCells:e.chartKind===`surface`?As(e,t):[],backend:`cpu`}}async function Ms(e,t){let n=Ha(e),r=e.chartKind===`curve`?Es():[],i=r.map(e=>[e]),a=e.chartKind===`surface`?ks(e):{cells:[],inputs:[]},o=await Ma([...n,...i,...a.inputs],t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}),s=o.predictions.map(e=>e[0]??0),c=s.slice(0,n.length),l=s.slice(n.length,n.length+i.length),u=s.slice(n.length+i.length);return{rowPredictions:c,curvePath:r.length>0?Ds(r,l):``,surfaceCells:a.cells.map((e,t)=>({...e,value:u[t]??0})),backend:o.backend,fallbackReason:o.fallbackReason}}function Ns({example:e,curvePath:t,predictions:n}){return(0,T.jsxs)(`svg`,{viewBox:`0 0 ${I.width} ${I.height}`,role:`img`,"aria-label":`${e.title} hidden-layer curve`,children:[(0,T.jsx)(`rect`,{className:`plot-bg`,x:I.padLeft,y:I.padTop,width:I.width-I.padLeft-I.padRight,height:I.height-I.padTop-I.padBottom}),[0,.25,.5,.75,1].map(e=>{let t=I.xMin+(I.xMax-I.xMin)*e,n=I.yMin+(I.yMax-I.yMin)*e;return(0,T.jsxs)(`g`,{children:[(0,T.jsx)(`line`,{className:`grid-line`,x1:Cs(t,I),x2:Cs(t,I),y1:I.padTop,y2:I.height-I.padBottom}),(0,T.jsx)(`text`,{className:`axis-label`,x:Cs(t,I),y:I.height-18,children:bs(t,1)}),(0,T.jsx)(`line`,{className:`grid-line`,x1:I.padLeft,x2:I.width-I.padRight,y1:ws(n,I),y2:ws(n,I)}),(0,T.jsx)(`text`,{className:`axis-label axis-label--y`,x:I.padLeft-10,y:ws(n,I)+4,children:bs(n,1)})]},e)}),(0,T.jsx)(`path`,{className:`hidden-curve`,d:t}),e.rows.map((e,t)=>{let r=Cs(e.input[0],I),i=ws(e.target,I),a=ws(n[t],I);return(0,T.jsxs)(`g`,{children:[(0,T.jsx)(`line`,{className:`error-line`,x1:r,x2:r,y1:i,y2:a}),(0,T.jsx)(`circle`,{className:`truth-point`,cx:r,cy:i,r:`6`}),(0,T.jsx)(`circle`,{className:`prediction-point`,cx:r,cy:a,r:`5`})]},e.label)}),(0,T.jsx)(`text`,{className:`axis-title`,x:I.width/2,y:I.height-5,children:e.inputLabels[0]}),(0,T.jsx)(`text`,{className:`axis-title axis-title--y`,x:`20`,y:I.height/2,children:e.outputLabel})]})}function Ps({example:e,cells:t,predictions:n,selectedIndex:r,onSelect:i}){let a=ys/Math.sqrt(t.length),o=e.rows.map(e=>e.input[0]),s=e.rows.map(e=>e.input[1]),c=Math.min(...o,-1),l=Math.max(...o,1),u=Math.min(...s,-1),d=Math.max(...s,1),f=Math.max((l-c)*.08,.15),p=Math.max((d-u)*.08,.15),m=e=>(e-(c-f))/(l-c+f*2)*ys,h=e=>ys-(e-(u-p))/(d-u+p*2)*ys;return(0,T.jsxs)(`svg`,{className:`surface-chart`,viewBox:`0 0 ${ys} ${ys}`,role:`img`,"aria-label":`${e.title} decision surface`,children:[t.map(e=>(0,T.jsx)(`rect`,{x:e.x*a,y:e.y*a,width:a+.5,height:a+.5,style:{fill:`rgba(${Math.round(194-e.value*150)}, ${Math.round(65+e.value*90)}, ${Math.round(59+e.value*120)}, 0.72)`}},`${e.x}-${e.y}`)),e.rows.map((e,t)=>(0,T.jsxs)(`g`,{"aria-label":`Select ${e.label}`,className:`svg-button`,role:`button`,tabIndex:0,onClick:()=>i(t),onKeyDown:e=>{(e.key===`Enter`||e.key===` `)&&i(t)},children:[(0,T.jsx)(`circle`,{className:t===r?`surface-point surface-point--selected`:`surface-point`,cx:m(e.input[0]),cy:h(e.input[1]),r:t===r?9:7,style:{fill:e.target>=.5?`#237a57`:`#f7f8f3`}}),(0,T.jsx)(`text`,{className:`surface-label`,x:m(e.input[0])+10,y:h(e.input[1])-8,children:bs(n[t],2)})]},e.label))]})}function Fs({example:e,predictions:t,selectedIndex:n,onSelect:r}){return(0,T.jsx)(`div`,{className:`hidden-table-chart`,children:e.rows.map((e,i)=>{let a=t[i]-e.target;return(0,T.jsxs)(`button`,{className:i===n?`table-row table-row--selected`:`table-row`,type:`button`,onClick:()=>r(i),children:[(0,T.jsx)(`span`,{children:e.label}),(0,T.jsxs)(`span`,{className:`bar-pair`,children:[(0,T.jsx)(`i`,{className:`bar-target`,style:{width:`${e.target*100}%`}}),(0,T.jsx)(`i`,{className:`bar-prediction`,style:{width:`${t[i]*100}%`}})]}),(0,T.jsx)(`code`,{children:bs(a,3)})]},e.label)})})}function Is(){let[e,t]=(0,l.useState)($a[0].id),n=$a.find(t=>t.id===e)??$a[0],[r,i]=(0,l.useState)(n.defaultLearningRate),[a,o]=(0,l.useState)(()=>Ua(n)),[s,c]=(0,l.useState)(()=>[qa(n,Ua(n))]),[u,d]=(0,l.useState)(null),[f,p]=(0,l.useState)(0),[m,h]=(0,l.useState)(!1),g=a.hiddenLayerCount;(0,l.useEffect)(()=>{let e=Ua(n);i(n.defaultLearningRate),o(e),c([qa(n,e)]),d(null),p(0),h(!1)},[n]);let _=(0,l.useMemo)(()=>js(n,a),[n,a]),[v,y]=(0,l.useState)(null),b=v??_,x=b.rowPredictions,ee=(0,l.useMemo)(()=>Ga(n,a),[n,a]),S=(0,l.useMemo)(()=>Ka(n,a),[n,a]),C=(0,l.useMemo)(()=>Xa(n,a,f),[n,f,a]),te=u?.step.weightGradients[u.step.weightGradients.length-1],ne=b.backend===`webgpu`?`WebGPU`:`CPU`;(0,l.useEffect)(()=>{let e=!1;return y(null),Na()&&Ms(n,a).then(t=>{e||y(t)}).catch(t=>{e||y({..._,fallbackReason:t instanceof Error?t.message:`Matrix backend failed`})}),()=>{e=!0}},[n,_,a]);function w(e){o(e.state),d(e),c(t=>[...t.slice(-159),{epoch:e.state.epoch,loss:e.loss,mae:e.mae}])}function re(e){let t=Ya(n,a,r,e),i=t[t.length-1];i!==void 0&&w(i)}function ie(){let e=Ua(n,g);o(e),c([qa(n,e)]),d(null),h(!1)}function ae(e){let n=Ua(e);t(e.id),i(e.defaultLearningRate),o(n),c([qa(e,n)]),d(null),p(0),h(!1)}function oe(e){let t=Ua(n,Math.max(n.hiddenLayerMin,Math.min(n.hiddenLayerMax,Math.round(e))));o(t),c([qa(n,t)]),d(null),p(0),h(!1)}return(0,l.useEffect)(()=>{if(!m)return;let e=window.setInterval(()=>{o(e=>{let t=Ya(n,e,r,5),i=t[t.length-1];return d(i),c(e=>[...e.slice(-159),{epoch:i.state.epoch,loss:i.loss,mae:i.mae}]),i.state})},160);return()=>window.clearInterval(e)},[n,m,r]),(0,T.jsxs)(`main`,{className:`workspace workspace--hidden`,children:[(0,T.jsxs)(`nav`,{className:`lab-rail`,"aria-label":`Hidden-layer examples`,children:[(0,T.jsxs)(`div`,{className:`rail-summary`,children:[(0,T.jsx)(`strong`,{children:$a.length}),(0,T.jsx)(`span`,{children:`hidden examples`})]}),(0,T.jsx)(`div`,{className:`lab-list`,children:$a.map(e=>(0,T.jsxs)(`button`,{className:e.id===n.id?`lab-button lab-button--active`:`lab-button`,type:`button`,onClick:()=>ae(e),children:[(0,T.jsx)(`span`,{children:e.title}),(0,T.jsx)(`small`,{children:e.category})]},e.id))})]}),(0,T.jsxs)(`section`,{className:`lab-stage`,"aria-label":`Hidden-layer training stage`,children:[(0,T.jsxs)(`div`,{className:`lab-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:n.category}),(0,T.jsx)(`h2`,{children:n.title}),(0,T.jsx)(`p`,{children:n.summary})]}),(0,T.jsxs)(`div`,{className:`lab-chip`,children:[g,` layers / `,n.hiddenCount,` neurons`]})]}),(0,T.jsxs)(`section`,{className:`chart-panel chart-panel--hidden`,"aria-label":`Hidden-layer chart`,children:[n.chartKind===`curve`&&(0,T.jsx)(Ns,{example:n,curvePath:b.curvePath,predictions:x}),n.chartKind===`surface`&&(0,T.jsx)(Ps,{example:n,cells:b.surfaceCells,predictions:x,selectedIndex:f,onSelect:p}),n.chartKind===`table`&&(0,T.jsx)(Fs,{example:n,predictions:x,selectedIndex:f,onSelect:p}),(0,T.jsxs)(`div`,{className:`legend`,"aria-label":`Hidden chart legend`,children:[(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-dot legend-dot--truth`}),`Target`]}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-dot legend-dot--prediction`}),`Prediction`]}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-line legend-line--model`}),`Current model`]})]})]}),(0,T.jsxs)(`section`,{className:`trace-panel`,"aria-label":`Neuron trace`,children:[(0,T.jsxs)(`div`,{className:`history__topline`,children:[(0,T.jsx)(`span`,{children:n.rows[f].label}),(0,T.jsxs)(`strong`,{children:[bs(x[f],3),` / `,bs(n.rows[f].target,3)]})]}),(0,T.jsx)(`div`,{className:`hidden-neuron-grid`,children:C.layers.filter(e=>e.layer.startsWith(`hidden`)).flatMap((e,t)=>e.neurons.map((e,n)=>(0,T.jsxs)(`div`,{className:`neuron-tile`,children:[(0,T.jsxs)(`span`,{children:[`h`,t+1,`.`,n+1]}),(0,T.jsx)(`strong`,{children:bs(e.output,3)}),(0,T.jsx)(`i`,{style:{width:`${xs(e.output,0,1)*100}%`}})]},e.neuron)))}),(0,T.jsx)(`div`,{className:`trace-equation`,children:(0,T.jsxs)(`code`,{children:[n.inputLabels.join(`, `),` `,`->`,` `,g,` x hidden[`,n.hiddenCount,`] `,`->`,` `,n.outputLabel]})})]}),(0,T.jsx)(Qo,{example:n,state:a,selectedRow:n.rows[f],selectedIndex:f,prediction:x[f],lastStep:u,learningRate:r})]}),(0,T.jsxs)(`aside`,{className:`controls metrics`,"aria-label":`Hidden-layer controls`,children:[(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Hidden layers`}),(0,T.jsx)(`input`,{type:`range`,min:n.hiddenLayerMin,max:n.hiddenLayerMax,step:`1`,value:g,onChange:e=>oe(Number(e.target.value))}),(0,T.jsx)(`input`,{type:`number`,min:n.hiddenLayerMin,max:n.hiddenLayerMax,step:`1`,value:g,onChange:e=>oe(Number(e.target.value))})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Learning rate`}),(0,T.jsx)(`input`,{type:`range`,min:n.learningRateMin,max:n.learningRateMax,step:n.learningRateStep,value:r,onChange:e=>i(Number(e.target.value))}),(0,T.jsx)(`input`,{type:`number`,min:n.learningRateMin,max:n.learningRateMax,step:n.learningRateStep,value:r,onChange:e=>i(Number(e.target.value))})]}),(0,T.jsxs)(`div`,{className:`button-grid`,children:[(0,T.jsx)(`button`,{type:`button`,onClick:()=>re(1),children:`Step`}),(0,T.jsx)(`button`,{type:`button`,onClick:()=>re(25),children:`Step 25`}),(0,T.jsx)(`button`,{type:`button`,onClick:()=>h(e=>!e),children:m?`Pause`:`Run`}),(0,T.jsx)(`button`,{type:`button`,onClick:ie,children:`Reset`})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Epoch`}),(0,T.jsx)(`strong`,{children:a.epoch})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Loss`}),(0,T.jsx)(`strong`,{children:bs(ee,4)})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Average error`}),(0,T.jsx)(`strong`,{children:bs(S,3)})]}),(0,T.jsxs)(`div`,{className:`metric`,title:b.fallbackReason,children:[(0,T.jsx)(`span`,{children:`Matrix backend`}),(0,T.jsx)(`strong`,{children:ne})]}),(0,T.jsxs)(`div`,{className:`history`,children:[(0,T.jsxs)(`div`,{className:`history__topline`,children:[(0,T.jsx)(`span`,{children:`Loss history`}),(0,T.jsxs)(`strong`,{children:[s.length,` points`]})]}),(0,T.jsxs)(`svg`,{viewBox:`0 0 250 74`,role:`img`,"aria-label":`Hidden-layer loss history`,children:[(0,T.jsx)(`path`,{className:`history-grid`,d:`M 0 37 L 250 37`}),(0,T.jsx)(`path`,{className:`history-line`,d:Ts(s)})]})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Trace row`}),(0,T.jsx)(`select`,{value:f,onChange:e=>p(Number(e.target.value)),children:n.rows.map((e,t)=>(0,T.jsx)(`option`,{value:t,children:e.label},e.label))})]}),(0,T.jsxs)(`div`,{className:`gradients`,children:[(0,T.jsx)(`span`,{children:`Last gradient shape`}),(0,T.jsx)(`code`,{children:Ss(g)}),(0,T.jsxs)(`code`,{children:[`input-hidden `,ka(u?.step.weightGradients[0])]}),(0,T.jsxs)(`code`,{children:[`hidden-output `,ka(te)]})]}),(0,T.jsxs)(`div`,{className:`lesson`,children:[(0,T.jsx)(`span`,{children:`Learning note`}),(0,T.jsx)(`p`,{children:n.lesson})]})]})]})}var Ls=[{name:`vertical-position`,values:[[0,0,0],[1,1,1],[2,2,2]]},{name:`horizontal-position`,values:[[0,1,2],[0,1,2],[0,1,2]]}],Rs=[{name:`toward-bottom-right`,kernels:[[[4,0],[0,0]],[[2,0],[0,0]]],bias:0},{name:`toward-top-left`,kernels:[[[-4,0],[0,0]],[[-2,0],[0,0]]],bias:6}],zs=[1,1],Bs=[0,0];function Vs(e){return e===0?0:e}function Hs(e){if(e.length===0||e[0].length===0)throw Error(`Matrices must contain at least one value.`);let t=e[0].length;if(e.some(e=>e.length!==t||!e.every(Number.isFinite)))throw Error(`Matrices must be rectangular and contain finite numbers.`);return[e.length,t]}function Us(e,t,n,r){if(!Number.isFinite(t)||t<=0)throw Error(`Normalization epsilon must be positive.`);if(n.length!==e.length||r.length!==e.length)throw Error(`Gamma and beta must match the output channel count.`);let i=[],a=[],o=[];return{means:i,variances:a,denominators:o,maps:e.map((e,s)=>{let c=e.flat(),l=c.reduce((e,t)=>e+t,0)/c.length,u=c.reduce((e,t)=>e+(t-l)**2,0)/c.length,d=Math.sqrt(u+t);return i.push(l),a.push(u),o.push(d),e.map(e=>e.map(e=>Vs(n[s]*(e-l)/d+r[s])))})}}function Ws(e){let t=[],n=[];for(let r of e){let e=-1/0,i=[0,0];for(let[t,n]of r.entries())for(let[r,a]of n.entries())a>e&&(e=a,i=[t,r]);t.push(e),n.push(i)}return{values:t,argmax:n}}function Gs(e=Ls,t=Rs,n=4,r=zs,i=Bs){if(e.length===0||t.length===0)throw Error(`The image and filter bank must be non-empty.`);let[a,o]=Hs(e[0].values);if(e.some(e=>{let t=Hs(e.values);return t[0]!==a||t[1]!==o}))throw Error(`Every input channel must have the same image shape.`);let s=[],c=[],l=[];for(let[n,r]of t.entries()){if(!Number.isFinite(r.bias)||r.kernels.length!==e.length)throw Error(`Every filter needs a finite bias and one kernel per input channel.`);let[t,i]=Hs(r.kernels[0]);if(r.kernels.some(e=>{let n=Hs(e);return n[0]!==t||n[1]!==i}))throw Error(`Every kernel in one filter must have the same shape.`);if(t>a||i>o)throw Error(`Kernels must fit inside the image in valid mode.`);let u=a-t+1,d=o-i+1,f=e.map(()=>Array.from({length:u},()=>Array(d).fill(0))),p=[],m=[];for(let a=0;a<u;a+=1){let o=[],s=[];for(let c=0;c<d;c+=1){let l=e.map(e=>Array.from({length:t},(t,n)=>e.values[a+n].slice(c,c+i))),u=l.map((e,t)=>e.map((e,n)=>e.map((e,i)=>Vs(e*r.kernels[t][n][i])))),d=u.map(e=>Vs(e.flat().reduce((e,t)=>e+t,0))),p=Vs(d.reduce((e,t)=>e+t,0)),m=Vs(p+r.bias);d.forEach((e,t)=>{f[t][a][c]=e}),o.push({filterIndex:n,row:a,column:c,windows:l,products:u,channelSums:d,preBiasSum:p,output:m}),s.push(m)}p.push(o),m.push(s)}s.push(p),c.push(f),l.push(m)}let u=Us(l,n,r,i),d=u.maps.map(e=>e.map(e=>e.map(e=>Math.max(0,e))));return{positions:s,channelContributions:c,convolution:l,normalization:u,activation:d,pooling:Ws(d)}}var Ks=[{id:`channels`,label:`Channels`},{id:`convolve`,label:`Convolve`},{id:`normalize`,label:`Normalize`},{id:`relu`,label:`ReLU`},{id:`pool`,label:`Pool`}];function qs(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function Js({values:e,label:t,selected:n,winner:r}){return(0,T.jsxs)(`div`,{className:`image-matrix-block`,children:[(0,T.jsx)(`span`,{children:t}),(0,T.jsx)(`div`,{className:`image-matrix`,style:{gridTemplateColumns:`repeat(${e[0].length}, minmax(44px, 1fr))`},"aria-label":t,children:e.flatMap((e,t)=>e.map((e,i)=>{let a=n?.[0]===t&&n[1]===i;return(0,T.jsxs)(`div`,{className:r?.[0]===t&&r[1]===i?`image-matrix-cell image-matrix-cell--winner`:a?`image-matrix-cell image-matrix-cell--selected`:`image-matrix-cell`,children:[(0,T.jsxs)(`small`,{children:[`[`,t,`,`,i,`]`]}),(0,T.jsx)(`strong`,{children:qs(e)})]},`${t}-${i}`)}))})]})}function Ys(){let[e,t]=(0,l.useState)(`channels`),[n,r]=(0,l.useState)(0),[i,a]=(0,l.useState)(3),o=(0,l.useMemo)(()=>Gs(),[]),s=Ks.findIndex(t=>t.id===e),c=Math.floor(i/2),u=i%2,d=Rs[n],f=o.positions[n][c][u],p=[c,u];function m(e){t(Ks[Math.min(Math.max(s+e,0),Ks.length-1)].id)}function h(){t(`channels`),r(0),a(3)}return(0,T.jsxs)(`main`,{className:`workspace workspace--image-cnn`,children:[(0,T.jsxs)(`section`,{className:`image-cnn-stage`,"aria-label":`Tiny image CNN trace`,children:[(0,T.jsxs)(`div`,{className:`image-cnn-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN07 · tiny image CNN`}),(0,T.jsx)(`h2`,{children:`Open the image pipeline`}),(0,T.jsx)(`p`,{children:`Follow two image channels through shared spatial windows, channel reduction, normalization, ReLU, and max pooling.`})]}),(0,T.jsx)(`div`,{className:`image-shape-chip`,children:`2 × 3 × 3 → 2 × 2 × 2 → 2`})]}),(0,T.jsx)(`nav`,{className:`image-pipeline`,"aria-label":`Image CNN pipeline stages`,children:Ks.map((n,r)=>(0,T.jsxs)(`button`,{"aria-label":`Show ${n.label} stage`,className:n.id===e?`image-stage-button image-stage-button--active`:r<s?`image-stage-button image-stage-button--visited`:`image-stage-button`,type:`button`,onClick:()=>t(n.id),children:[(0,T.jsx)(`small`,{children:r+1}),(0,T.jsx)(`strong`,{children:n.label})]},n.id))}),e===`channels`?(0,T.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Input image channels`,children:[(0,T.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Stage 1 · input tensor`}),(0,T.jsx)(`h2`,{children:`One image can have several number grids`})]}),(0,T.jsx)(`code`,{children:`shape [channels, rows, columns] = [2, 3, 3]`})]}),(0,T.jsx)(`div`,{className:`image-channel-grid`,children:Ls.map((e,t)=>(0,T.jsxs)(`article`,{className:`image-channel-card`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`input channel `,t]}),(0,T.jsx)(`strong`,{children:e.name})]}),(0,T.jsx)(Js,{values:e.values,label:`${e.name} values`})]},e.name))}),(0,T.jsx)(`p`,{className:`image-stage-note`,children:`A filter owns one kernel per input channel. Their spatial results meet only after each channel has produced its own partial sum.`})]}):null,e===`convolve`?(0,T.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Channel accumulation trace`,children:[(0,T.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Stage 2 · filter `,n,` · output [`,c,`,`,u,`]`]}),(0,T.jsx)(`h2`,{children:`Correlate each channel, then add`})]}),(0,T.jsx)(`strong`,{className:`image-output-value`,children:qs(f.output)})]}),(0,T.jsx)(`div`,{className:`channel-math-grid`,children:Ls.map((e,t)=>(0,T.jsxs)(`article`,{className:`channel-math-card`,children:[(0,T.jsxs)(`div`,{className:`channel-math-title`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`channel `,t]}),(0,T.jsx)(`strong`,{children:e.name})]}),(0,T.jsx)(`strong`,{children:qs(f.channelSums[t])})]}),(0,T.jsxs)(`div`,{className:`window-kernel-pair`,children:[(0,T.jsx)(Js,{values:f.windows[t],label:`selected window`}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`×`}),(0,T.jsx)(Js,{values:d.kernels[t],label:`channel kernel`})]}),(0,T.jsx)(`div`,{className:`image-product-list`,children:f.products[t].flatMap((e,n)=>e.map((e,r)=>(0,T.jsxs)(`code`,{children:[qs(f.windows[t][n][r]),`×`,qs(d.kernels[t][n][r]),`=`,qs(e)]},`${n}-${r}`)))})]},e.name))}),(0,T.jsxs)(`div`,{className:`channel-reduction`,"aria-label":`Channel reduction equation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`channel 0`}),(0,T.jsx)(`strong`,{children:qs(f.channelSums[0])})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`channel 1`}),(0,T.jsx)(`strong`,{children:qs(f.channelSums[1])})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias`}),(0,T.jsx)(`strong`,{children:qs(d.bias)})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{className:`channel-reduction__result`,children:[(0,T.jsx)(`small`,{children:`output`}),(0,T.jsx)(`strong`,{children:qs(f.output)})]})]}),(0,T.jsx)(`div`,{className:`image-map-pair`,children:o.convolution.map((e,t)=>(0,T.jsx)(Js,{values:e,label:`filter ${t} convolution map`,selected:t===n?p:void 0},t))})]}):null,e===`normalize`?(0,T.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Spatial normalization trace`,children:[(0,T.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Stage 3 · output channel `,n]}),(0,T.jsx)(`h2`,{children:`Four spatial values share statistics`})]}),(0,T.jsx)(`code`,{children:`(x − μ) / √(variance + ε)`})]}),(0,T.jsxs)(`div`,{className:`normalization-flow`,children:[(0,T.jsx)(Js,{values:o.convolution[n],label:`convolution map`,selected:p}),(0,T.jsxs)(`div`,{className:`normalization-stats`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean μ`}),(0,T.jsx)(`strong`,{children:qs(o.normalization.means[n])})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`variance`}),(0,T.jsx)(`strong`,{children:qs(o.normalization.variances[n])})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`epsilon ε`}),(0,T.jsx)(`strong`,{children:qs(4)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`denominator`}),(0,T.jsx)(`strong`,{children:qs(o.normalization.denominators[n])})]})]}),(0,T.jsx)(Js,{values:o.normalization.maps[n],label:`normalized map`,selected:p})]}),(0,T.jsxs)(`code`,{className:`normalization-equation`,children:[`(`,qs(f.output),` − `,qs(o.normalization.means[n]),`)`,` `,`/ `,qs(o.normalization.denominators[n]),` `,`× γ `,qs(zs[n]),` `,`+ β `,qs(Bs[n]),` `,`= `,qs(o.normalization.maps[n][c][u])]})]}):null,e===`relu`?(0,T.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`ReLU activation trace`,children:[(0,T.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Stage 4 · output channel `,n]}),(0,T.jsx)(`h2`,{children:`Keep positive evidence`})]}),(0,T.jsx)(`code`,{children:`ReLU(x) = max(0, x)`})]}),(0,T.jsxs)(`div`,{className:`activation-flow`,children:[(0,T.jsx)(Js,{values:o.normalization.maps[n],label:`normalized values`,selected:p}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,T.jsx)(Js,{values:o.activation[n],label:`after ReLU`,selected:p})]}),(0,T.jsxs)(`code`,{className:`normalization-equation`,children:[`max(0, `,qs(o.normalization.maps[n][c][u]),`)`,` `,`= `,qs(o.activation[n][c][u])]})]}):null,e===`pool`?(0,T.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Max pooling trace`,children:[(0,T.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Stage 5 · shrink the maps`}),(0,T.jsx)(`h2`,{children:`Keep each channel's strongest location`})]}),(0,T.jsx)(`code`,{children:`2 × 2 max pool · stride 2`})]}),(0,T.jsx)(`div`,{className:`pooling-grid`,children:o.activation.map((e,t)=>(0,T.jsxs)(`article`,{className:`pooling-card`,children:[(0,T.jsx)(Js,{values:e,label:`filter ${t} activated map`,winner:o.pooling.argmax[t]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,T.jsxs)(`div`,{className:`pooled-value`,children:[(0,T.jsxs)(`small`,{children:[`pooled[`,t,`]`]}),(0,T.jsx)(`strong`,{children:qs(o.pooling.values[t])}),(0,T.jsxs)(`code`,{children:[`from [`,o.pooling.argmax[t][0],`,`,o.pooling.argmax[t][1],`]`]})]})]},t))}),(0,T.jsx)(`p`,{className:`image-stage-note`,children:`Only the highlighted winner receives gradient through max pooling. The other three values were useful for comparison, but are discarded.`})]}):null]}),(0,T.jsxs)(`aside`,{className:`image-cnn-controls`,"aria-label":`Image CNN trace controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Choose one path`}),(0,T.jsx)(`h2`,{children:`Filter and output`}),(0,T.jsx)(`p`,{children:`Selections stay synchronized as you move through the pipeline.`}),(0,T.jsxs)(`div`,{className:`image-control-group`,children:[(0,T.jsx)(`span`,{children:`Output filter`}),(0,T.jsx)(`div`,{className:`image-filter-buttons`,children:Rs.map((e,t)=>(0,T.jsxs)(`button`,{"aria-label":`Select filter ${t} ${e.name}`,className:t===n?`image-choice image-choice--active`:`image-choice`,type:`button`,onClick:()=>r(t),children:[(0,T.jsxs)(`small`,{children:[`filter `,t]}),(0,T.jsx)(`strong`,{children:e.name})]},e.name))})]}),(0,T.jsxs)(`div`,{className:`image-control-group`,children:[(0,T.jsx)(`span`,{children:`Spatial output`}),(0,T.jsx)(`div`,{className:`image-position-buttons`,children:[0,1,2,3].map(e=>{let t=Math.floor(e/2),r=e%2;return(0,T.jsxs)(`button`,{"aria-label":`Select image output row ${t} column ${r}`,className:e===i?`image-choice image-choice--active`:`image-choice`,type:`button`,onClick:()=>a(e),children:[(0,T.jsxs)(`small`,{children:[`[`,t,`,`,r,`]`]}),(0,T.jsx)(`strong`,{children:qs(o.convolution[n][t][r])})]},e)})})]}),(0,T.jsxs)(`div`,{className:`button-grid image-stage-controls`,children:[(0,T.jsx)(`button`,{type:`button`,disabled:s===0,onClick:()=>m(-1),children:`Previous stage`}),(0,T.jsx)(`button`,{type:`button`,disabled:s===Ks.length-1,onClick:()=>m(1),children:`Next stage`}),(0,T.jsx)(`button`,{type:`button`,onClick:h,children:`Reset trace`})]}),(0,T.jsxs)(`div`,{className:`image-cnn-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Larger CNNs repeat these same loops over batches, many channels, many filters, and deeper feature maps. Accelerators change the schedule, not the arithmetic contract.`})]})]})]})}var Xs=`species,island,bill_length_mm,bill_depth_mm,flipper_length_mm,body_mass_g,sex,year
Adelie,Torgersen,39.1,18.7,181,3750,male,2007
Adelie,Torgersen,39.5,17.4,186,3800,female,2007
Adelie,Torgersen,40.3,18,195,3250,female,2007
Adelie,Torgersen,36.7,19.3,193,3450,female,2007
Adelie,Torgersen,39.3,20.6,190,3650,male,2007
Adelie,Torgersen,38.9,17.8,181,3625,female,2007
Adelie,Torgersen,39.2,19.6,195,4675,male,2007
Adelie,Torgersen,41.1,17.6,182,3200,female,2007
Adelie,Torgersen,38.6,21.2,191,3800,male,2007
Adelie,Torgersen,34.6,21.1,198,4400,male,2007
Adelie,Torgersen,36.6,17.8,185,3700,female,2007
Adelie,Torgersen,38.7,19,195,3450,female,2007
Adelie,Torgersen,42.5,20.7,197,4500,male,2007
Adelie,Torgersen,34.4,18.4,184,3325,female,2007
Adelie,Torgersen,46,21.5,194,4200,male,2007
Adelie,Biscoe,37.8,18.3,174,3400,female,2007
Adelie,Biscoe,37.7,18.7,180,3600,male,2007
Adelie,Biscoe,35.9,19.2,189,3800,female,2007
Adelie,Biscoe,38.2,18.1,185,3950,male,2007
Adelie,Biscoe,38.8,17.2,180,3800,male,2007
Adelie,Biscoe,35.3,18.9,187,3800,female,2007
Adelie,Biscoe,40.6,18.6,183,3550,male,2007
Adelie,Biscoe,40.5,17.9,187,3200,female,2007
Adelie,Biscoe,37.9,18.6,172,3150,female,2007
Adelie,Biscoe,40.5,18.9,180,3950,male,2007
Adelie,Dream,39.5,16.7,178,3250,female,2007
Adelie,Dream,37.2,18.1,178,3900,male,2007
Adelie,Dream,39.5,17.8,188,3300,female,2007
Adelie,Dream,40.9,18.9,184,3900,male,2007
Adelie,Dream,36.4,17,195,3325,female,2007
`,Zs=[`species`,`island`,`bill_length_mm`,`bill_depth_mm`,`flipper_length_mm`,`body_mass_g`,`sex`,`year`];function Qs(e){return Number(e)}function $s(e){return e.trim().split(`
`).slice(1).map(e=>{let t=e.split(`,`),n=Object.fromEntries(Zs.map((e,n)=>[e,t[n]??``]));return{species:n.species,island:n.island,bill_length_mm:Qs(n.bill_length_mm),bill_depth_mm:Qs(n.bill_depth_mm),flipper_length_mm:Qs(n.flipper_length_mm),body_mass_g:Qs(n.body_mass_g),sex:n.sex,year:Qs(n.year)}}).filter(e=>Number.isFinite(e.bill_length_mm)&&Number.isFinite(e.body_mass_g))}var ec=$s(Xs);function tc(e,t){return ec.map(n=>({x:n[e],y:n[t],label:`${n.species} on ${n.island}`,group:n.species}))}function nc(e,t){return Aa(e.map(e=>e.x),t).predictions}function rc(e,t,n){let r=nc(e,t);return n===`mse`?e.reduce((e,t,n)=>{let i=r[n]-t.y;return e+i*i},0)/e.length:e.reduce((e,t,n)=>e+Math.abs(r[n]-t.y),0)/e.length}function ic(e,t){return rc(e,t,`mae`)}function ac(e,t,n){let r=nc(e,t),i=e.length;return e.reduce((e,t,a)=>{let o=r[a]-t.y,s=n===`mse`?2/i*o:Math.sign(o)/i;return{gradientWeight:e.gradientWeight+s*t.x,gradientBias:e.gradientBias+s}},{gradientWeight:0,gradientBias:0})}function oc(e,t,n,r){let{gradientWeight:i,gradientBias:a}=ac(e,t,r),o=rc(e,t,r),s={weight:t.weight-n*i,bias:t.bias-n*a,epoch:t.epoch+1};return{previousState:t,previousLoss:o,state:s,loss:rc(e,s,r),mae:ic(e,s),gradientWeight:i,gradientBias:a}}function sc(e,t,n,r,i){let a=[],o=t;for(let t=0;t<i;t+=1){let t=oc(e,o,n,r);a.push(t),o=t.state}return a}function cc(e){let t=e.length,n=e.reduce((e,t)=>e+t.x,0)/t,r=e.reduce((e,t)=>e+t.y,0)/t,i=e.reduce((e,t)=>e+(t.x-n)*(t.y-r),0),a=e.reduce((e,t)=>e+(t.x-n)**2,0),o=a===0?0:i/a;return{weight:o,bias:r-o*n,epoch:0}}var lc={name:`Generated in browser from deterministic formulas`,kind:`synthetic`,license:`Generated example data`},uc={name:`Palmer Penguins sample`,kind:`local-csv`,license:`CC0 1.0 Universal`,url:`https://github.com/allisonhorst/palmerpenguins`},dc=[-8,-6,-4,-2,0,2,4,6,8],fc=[-40,-10,0,8,15,22,38,60,100],pc=[0,.12,.25,.38,.5,.62,.75,.88,1];function mc(e){return e.toLowerCase().replace(/[^a-z0-9]+/g,`-`).replace(/^-|-$/g,``)}function hc(e,t,n={}){let r=n.xs??dc,i=n.noise??0,a=n.seed??1,o=n.curve??0;return r.map((r,s)=>{let c=Math.sin((s+1)*(a+1.7))*i,l=s===n.outlierIndex?n.outlierShift??0:0;return{x:r,y:e*r+t+o*r*r+c+l}})}function gc(e){let t=cc(e.points),n=e.defaultLearningRate??.01;return{id:mc(`${e.category}-${e.title}`),title:e.title,category:e.category,summary:e.summary,lesson:e.lesson,xLabel:e.xLabel??`Input`,yLabel:e.yLabel??`Target`,points:e.points,defaultLoss:e.defaultLoss??`mse`,defaultLearningRate:n,learningRateMin:n/20,learningRateMax:n*40,learningRateStep:n/20,initialModel:e.initialModel??{weight:0,bias:0,epoch:0},idealModel:t,source:e.source??lc}}var _c=[[`Celsius to Fahrenheit`,`Exact unit conversion with a real slope and intercept.`,1.8,32,fc,5e-4],[`Inches to centimeters`,`A clean proportional relationship with almost no intercept.`,2.54,0,dc,.01],[`Miles to kilometers`,`Another unit conversion where the slope carries the lesson.`,1.609,0,dc,.01],[`Hours to wages`,`A wage model where the intercept acts like a fixed bonus.`,18,40,dc,.002],[`Study time to quiz score`,`A friendly positive trend with a meaningful baseline.`,6,52,dc,.005],[`Screen brightness to battery draw`,`A line where increasing input increases cost.`,.42,1.2,dc,.02],[`Discount to final price`,`A negative slope: more discount means lower price.`,-.8,100,dc,.004],[`Altitude to air temperature`,`A negative physical trend with an intercept.`,-3.5,70,dc,.006],[`Recipe servings to flour`,`A proportional recipe scaling example.`,120,0,pc,.02],[`Parking time to fee`,`A simple line with a starting fee and per-hour growth.`,3.5,4,dc,.012]],vc=Array.from({length:15},(e,t)=>{let n=[2e-4,5e-4,.001,.002,.004][t%5],r=1.2+t%3*.45;return gc({title:`Learning rate ${t+1}: ${n}`,category:`Learning rate`,summary:`Compare how step size changes convergence speed and stability.`,lesson:`A useful learning rate moves downhill visibly without bouncing across the valley.`,points:hc(r,8+t,{xs:fc,noise:t%2==0?0:2,seed:t}),defaultLearningRate:n})}),yc=Array.from({length:15},(e,t)=>{let n=t%2==1;return gc({title:`${n?`Outlier`:`Clean`} loss comparison ${t+1}`,category:`Loss functions`,summary:`Switch between MSE and MAE to see how error shape changes the update.`,lesson:`MSE squares large mistakes, so a single bad point can pull the fitted line harder than MAE.`,points:hc(2.4,12,{xs:dc,noise:.8+t%4*.4,seed:t+2,outlierIndex:n?7:void 0,outlierShift:n?22+t:0}),defaultLoss:n?`mae`:`mse`,defaultLearningRate:.008})}),bc=Array.from({length:15},(e,t)=>{let n=t%3==0?pc:t%3==1?dc:fc,r=t%3==0?`normalized`:t%3==1?`centered`:`wide`;return gc({title:`Feature scale ${t+1}: ${r}`,category:`Scaling`,summary:`The same visual idea becomes easier or harder to optimize depending on input scale.`,lesson:`Large input values make gradients large; normalized inputs usually tolerate larger learning rates.`,points:hc(1.1+t*.08,4,{xs:n,noise:.5,seed:t+4}),defaultLearningRate:r===`wide`?6e-4:.015})}),xc=Array.from({length:15},(e,t)=>{let n=.5+t*.5;return gc({title:`Noise level ${t+1}`,category:`Noise`,summary:`Watch the line chase a pattern when the points stop landing exactly on it.`,lesson:`Noise means the best line is not the line through every point; it is the line that balances errors.`,points:hc(3.1,-6,{noise:n,seed:t+6}),defaultLearningRate:.007})}),Sc=Array.from({length:12},(e,t)=>{let n=t%2==0;return gc({title:`${n?`Curved data`:`Sparse data`} ${t+1}`,category:`Generalization`,summary:`Use a line even when the world is not perfectly linear.`,lesson:n?`A linear model can still be useful on curved data, but the residuals reveal its limits.`:`With only a few points, the line can look confident while still being fragile.`,points:hc(1.7,5,{xs:n?dc:[-8,-2,1,7],noise:.7,seed:t+8,curve:n?.12+t*.01:0}),defaultLearningRate:.007})}),Cc=[[`flipper_length_mm`,`body_mass_g`,`Flipper length to body mass`,`Longer flippers usually come with larger body mass.`],[`bill_length_mm`,`body_mass_g`,`Bill length to body mass`,`Bill length has signal, but the relationship is messier.`],[`bill_depth_mm`,`body_mass_g`,`Bill depth to body mass`,`A weak feature shows why not every measurement predicts well.`],[`flipper_length_mm`,`bill_length_mm`,`Flipper length to bill length`,`A moderate trend shows shared body-size information.`],[`bill_length_mm`,`bill_depth_mm`,`Bill length to bill depth`,`This relationship is noisy because species mix differently.`],[`year`,`body_mass_g`,`Observation year to body mass`,`A poor predictor is useful because the loss does not improve much.`]].flatMap(([e,t,n,r])=>[`MSE view`,`MAE view`,`small learning rate`].map((i,a)=>gc({title:`${n}: ${i}`,category:`Real data`,summary:`A checked-in CC0 CSV sample from Palmer Penguins, used without runtime network loading.`,lesson:r,xLabel:e.replaceAll(`_`,` `),yLabel:t.replaceAll(`_`,` `),points:tc(e,t),defaultLoss:a===1?`mae`:`mse`,defaultLearningRate:a===2?4e-7:1e-6,initialModel:{weight:0,bias:3e3,epoch:0},source:uc}))),wc=[..._c.map(([e,t,n,r,i,a])=>gc({title:e,category:`Basics`,summary:t,lesson:`Start with simple lines so weight, bias, prediction, error, and loss become visible.`,xLabel:e===`Celsius to Fahrenheit`?`Celsius`:`Input`,yLabel:e===`Celsius to Fahrenheit`?`Fahrenheit`:`Target`,points:hc(n,r,{xs:i}),defaultLearningRate:a,initialModel:e===`Celsius to Fahrenheit`?{weight:.5,bias:.5,epoch:0}:void 0})),...vc,...yc,...bc,...xc,...Sc,...Cc],Tc=[`Basics`,`Learning rate`,`Loss functions`,`Scaling`,`Noise`,`Generalization`,`Real data`],Ec=[{x:-1,y:-1},{x:0,y:1},{x:1,y:3},{x:2,y:5}],Dc={weight:-.5,bias:0,step:0},Oc={weight:2,bias:1};function kc(e,t){return t.weight*e.x+t.bias}function Ac(e,t){if(e.length===0)throw Error(`meanSquaredError requires at least one point`);return e.reduce((e,n)=>{let r=kc(n,t)-n.y;return e+r*r},0)/e.length}function jc(e,t){if(e.length===0)throw Error(`analyticalGradient requires at least one point`);let n=2/e.length;return e.reduce((e,r)=>{let i=kc(r,t)-r.y;return{weight:e.weight+n*i*r.x,bias:e.bias+n*i}},{weight:0,bias:0})}function Mc(e,t,n){if(!(n>0)||!Number.isFinite(n))throw Error(`epsilon must be a positive finite number`);function r(r){let i={...t,[r]:t[r]+n},a={...t,[r]:t[r]-n};return(Ac(e,i)-Ac(e,a))/(2*n)}return{weight:r(`weight`),bias:r(`bias`)}}function Nc(e,t,n,r=1e-6){let i=jc(e,t),a=Mc(e,t,n),o={weight:Math.abs(i.weight-a.weight),bias:Math.abs(i.bias-a.bias)},s=[`weight`,`bias`].map(e=>{let t=Math.max(1,Math.abs(i[e]),Math.abs(a[e]));return o[e]/t}),c=Math.max(...s);return{analytical:i,numerical:a,absoluteError:o,maximumRelativeError:c,passes:c<=r}}function Pc(e,t,n){if(!Number.isInteger(n)||n<1)throw Error(`pointCount must be a positive integer`);if(e===`full-batch`)return Array.from({length:n},(e,t)=>t);if(e===`stochastic`)return[t%n];let r=t*2%n;return[r,(r+1)%n]}function Fc(e,t,n,r){if(!(n>0)||!Number.isFinite(n))throw Error(`learningRate must be a positive finite number`);let i=Pc(r,t.step,e.length),a=jc(i.map(t=>e[t]),t),o={weight:t.weight-n*a.weight,bias:t.bias-n*a.bias,step:t.step+1};return{...o,loss:Ac(e,o),batchIndices:i}}function Ic(e,t,n,r=Dc,i=Ec){if(!Number.isInteger(t)||t<0)throw Error(`steps must be a non-negative integer`);let a=[{...r,loss:Ac(i,r),batchIndices:[]}],o=r;for(let r=0;r<t;r+=1){let t=Fc(i,o,n,e);a.push(t),o=t}return a}function Lc(e,t,n,r){if(!Number.isInteger(r)||r<2)throw Error(`resolution must be an integer of at least two`);let i=[];for(let a=0;a<r;a+=1){let o=n[0]+(n[1]-n[0])*(a/(r-1));for(let n=0;n<r;n+=1){let s=t[0]+(t[1]-t[0])*(n/(r-1));i.push({weight:s,bias:o,loss:Ac(e,{weight:s,bias:o,step:0}),column:n,row:a})}}return i}var Rc=[{kind:`stochastic`,label:`SGD / 1 row`,summary:`Noisy, frequent updates`},{kind:`mini-batch`,label:`Mini-batch / 2 rows`,summary:`A compromise between noise and stability`},{kind:`full-batch`,label:`Full batch / 4 rows`,summary:`Stable average gradient`}],L={width:720,height:430,left:68,right:28,top:24,bottom:58,weightRange:[-1,3.5],biasRange:[-1,3],resolution:25};function zc(e,t=5){return Math.abs(e)<1e-12?`0`:Math.abs(e)>=1e3||Math.abs(e)<1e-4?e.toExponential(3):Number(e.toFixed(t)).toString()}function Bc(e){let t=L.width-L.left-L.right;return L.left+(e-L.weightRange[0])/(L.weightRange[1]-L.weightRange[0])*t}function Vc(e){let t=L.height-L.top-L.bottom;return L.top+(1-(e-L.biasRange[0])/(L.biasRange[1]-L.biasRange[0]))*t}function Hc(e,t){if(e.length===0)return``;let n=Math.max(e.length-1,1);return e.map((e,r)=>{let i=r/n*590,a=138-Math.log1p(e.loss)/t*138;return`${r===0?`M`:`L`} ${i.toFixed(2)} ${a.toFixed(2)}`}).join(` `)}function Uc(e,t){let n=Number(e);return Number.isFinite(n)?n:t}function Wc(){let[e,t]=(0,l.useState)(Dc),[n,r]=(0,l.useState)(1e-5),[i,a]=(0,l.useState)(.05),[o,s]=(0,l.useState)(20),c=(0,l.useMemo)(()=>Nc(Ec,e,n),[n,e]),u=(0,l.useMemo)(()=>Lc(Ec,L.weightRange,L.biasRange,L.resolution),[]),d=(0,l.useMemo)(()=>Math.max(...u.map(e=>Math.log1p(e.loss)),1),[u]),f=(0,l.useMemo)(()=>Rc.map(t=>({...t,trace:Ic(t.kind,o,i,e)})),[i,e,o]),p=Math.max(...f.flatMap(e=>e.trace.map(e=>Math.log1p(e.loss))),1),m=Fc(Ec,e,i,`full-batch`),h=(L.width-L.left-L.right)/L.resolution,g=(L.height-L.top-L.bottom)/L.resolution;function _(e,n){t(t=>({...t,[e]:Uc(n,t[e]),step:0}))}function v(){t(Dc),r(1e-5),a(.05),s(20)}return(0,T.jsxs)(`main`,{className:`workspace workspace--optimization`,children:[(0,T.jsxs)(`section`,{className:`optimization-stage`,"aria-label":`Optimization microscope`,children:[(0,T.jsxs)(`div`,{className:`lab-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Slope / check / step size / batch noise`}),(0,T.jsx)(`h2`,{children:`Optimization microscope`}),(0,T.jsx)(`p`,{children:`See the loss surface, verify the gradient independently, and compare three ways to choose training rows.`})]}),(0,T.jsxs)(`div`,{className:`lab-chip`,children:[`MSE `,zc(Ac(Ec,e),4)]})]}),(0,T.jsxs)(`section`,{className:`landscape-panel`,"aria-label":`Loss landscape`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Every location is one model`}),(0,T.jsx)(`h2`,{children:`Loss landscape`})]}),(0,T.jsx)(`span`,{children:`Darker = larger loss`})]}),(0,T.jsxs)(`svg`,{className:`landscape-chart`,viewBox:`0 0 ${L.width} ${L.height}`,role:`img`,"aria-label":`Mean squared error by weight and bias. Current weight ${zc(e.weight)} and bias ${zc(e.bias)}.`,children:[(0,T.jsx)(`title`,{children:`Loss landscape for a four-point linear regression problem`}),u.map(e=>(0,T.jsx)(`rect`,{className:`landscape-cell`,x:L.left+e.column*h,y:L.top+(L.resolution-1-e.row)*g,width:h+.4,height:g+.4,style:{opacity:.08+.78*(Math.log1p(e.loss)/d)}},`${e.row}-${e.column}`)),(0,T.jsx)(`line`,{className:`gradient-arrow`,x1:Bc(e.weight),y1:Vc(e.bias),x2:Bc(m.weight),y2:Vc(m.bias),markerEnd:`url(#gradient-arrow-head)`}),(0,T.jsx)(`defs`,{children:(0,T.jsx)(`marker`,{id:`gradient-arrow-head`,markerWidth:`8`,markerHeight:`8`,refX:`5`,refY:`3`,orient:`auto`,children:(0,T.jsx)(`path`,{d:`M 0 0 L 6 3 L 0 6 z`,className:`gradient-arrow-head`})})}),(0,T.jsx)(`circle`,{className:`optimum-point`,cx:Bc(Oc.weight),cy:Vc(Oc.bias),r:`8`}),(0,T.jsx)(`text`,{className:`landscape-label`,x:Bc(Oc.weight)+12,y:Vc(Oc.bias)-10,children:`minimum (2, 1)`}),(0,T.jsx)(`circle`,{className:`current-parameter-point`,cx:Bc(e.weight),cy:Vc(e.bias),r:`9`}),(0,T.jsx)(`text`,{className:`landscape-label`,x:Bc(e.weight)+12,y:Vc(e.bias)+22,children:`current model`}),(0,T.jsx)(`text`,{className:`axis-title`,x:L.width/2,y:L.height-10,children:`weight w`}),(0,T.jsx)(`text`,{className:`axis-title axis-title--optimization-y`,x:`18`,y:L.height/2,children:`bias b`})]}),(0,T.jsxs)(`div`,{className:`landscape-equation`,children:[(0,T.jsxs)(`code`,{children:[`w' = `,zc(e.weight),` - `,zc(i),` x (`,zc(c.analytical.weight),`) = `,zc(m.weight)]}),(0,T.jsxs)(`code`,{children:[`b' = `,zc(e.bias),` - `,zc(i),` x (`,zc(c.analytical.bias),`) = `,zc(m.bias)]})]})]}),(0,T.jsxs)(`section`,{className:`gradient-check-panel`,"aria-label":`Finite-difference gradient check`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Backpropagation gets an independent audit`}),(0,T.jsx)(`h2`,{children:`Finite-difference gradient check`})]}),(0,T.jsx)(`span`,{className:c.passes?`check-status check-status--pass`:`check-status check-status--fail`,children:c.passes?`PASS`:`CHECK EPSILON`})]}),(0,T.jsxs)(`div`,{className:`gradient-check-grid`,role:`table`,"aria-label":`Gradient comparison`,children:[(0,T.jsx)(`span`,{role:`columnheader`,children:`Parameter`}),(0,T.jsx)(`span`,{role:`columnheader`,children:`Backprop`}),(0,T.jsx)(`span`,{role:`columnheader`,children:`Finite difference`}),(0,T.jsx)(`span`,{role:`columnheader`,children:`Absolute error`}),(0,T.jsx)(`strong`,{role:`cell`,children:`weight`}),(0,T.jsx)(`code`,{role:`cell`,children:zc(c.analytical.weight)}),(0,T.jsx)(`code`,{role:`cell`,children:zc(c.numerical.weight)}),(0,T.jsx)(`code`,{role:`cell`,children:zc(c.absoluteError.weight)}),(0,T.jsx)(`strong`,{role:`cell`,children:`bias`}),(0,T.jsx)(`code`,{role:`cell`,children:zc(c.analytical.bias)}),(0,T.jsx)(`code`,{role:`cell`,children:zc(c.numerical.bias)}),(0,T.jsx)(`code`,{role:`cell`,children:zc(c.absoluteError.bias)})]}),(0,T.jsx)(`p`,{children:`Finite differences nudge one parameter by +/- epsilon and estimate the slope without using backpropagation.`})]}),(0,T.jsxs)(`section`,{className:`batch-comparison-panel`,"aria-label":`Batch strategy comparison`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same model / same data / different row selection`}),(0,T.jsx)(`h2`,{children:`Batch versus stochastic updates`})]}),(0,T.jsxs)(`span`,{children:[o,` updates`]})]}),(0,T.jsxs)(`svg`,{className:`batch-chart`,viewBox:`0 0 650 175`,role:`img`,"aria-label":`Loss trajectories for stochastic, mini-batch, and full-batch gradient descent`,children:[(0,T.jsx)(`line`,{className:`batch-grid`,x1:`42`,x2:`632`,y1:`148`,y2:`148`}),(0,T.jsx)(`line`,{className:`batch-grid`,x1:`42`,x2:`42`,y1:`10`,y2:`148`}),(0,T.jsx)(`g`,{transform:`translate(42 10)`,children:f.map(e=>(0,T.jsx)(`path`,{className:`batch-line batch-line--${e.kind}`,d:Hc(e.trace,p)},e.kind))}),(0,T.jsx)(`text`,{className:`batch-axis-label`,x:`337`,y:`172`,children:`update`}),(0,T.jsx)(`text`,{className:`batch-axis-label batch-axis-label--y`,x:`12`,y:`82`,children:`log loss`})]}),(0,T.jsx)(`div`,{className:`strategy-grid`,children:f.map(e=>{let t=e.trace[e.trace.length-1];return(0,T.jsxs)(`div`,{className:`strategy-summary strategy-summary--${e.kind}`,children:[(0,T.jsx)(`strong`,{children:e.label}),(0,T.jsx)(`span`,{children:e.summary}),(0,T.jsxs)(`code`,{children:[`loss `,zc(t.loss,4)]}),(0,T.jsxs)(`small`,{children:[`w `,zc(t.weight,3),` / b `,zc(t.bias,3)]})]},e.kind)})})]})]}),(0,T.jsxs)(`aside`,{className:`controls optimization-controls`,"aria-label":`Optimization controls`,children:[(0,T.jsxs)(`div`,{className:`lesson`,children:[(0,T.jsx)(`span`,{children:`Try this`}),(0,T.jsx)(`p`,{children:`Move the model away from the minimum, then increase the learning rate until one or more trajectories overshoot.`})]}),(0,T.jsxs)(`div`,{className:`field-grid`,children:[(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Weight w`}),(0,T.jsx)(`input`,{"aria-label":`Optimization weight`,type:`number`,step:`0.1`,value:e.weight,onChange:e=>_(`weight`,e.target.value)})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Bias b`}),(0,T.jsx)(`input`,{"aria-label":`Optimization bias`,type:`number`,step:`0.1`,value:e.bias,onChange:e=>_(`bias`,e.target.value)})]})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Learning rate`}),(0,T.jsx)(`input`,{"aria-label":`Optimization learning rate`,type:`range`,min:`0.005`,max:`0.3`,step:`0.005`,value:i,onChange:e=>a(Number(e.target.value))}),(0,T.jsx)(`input`,{type:`number`,min:`0.005`,max:`0.3`,step:`0.005`,value:i,onChange:e=>a(Uc(e.target.value,i))})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Comparison updates`}),(0,T.jsx)(`input`,{"aria-label":`Comparison updates`,type:`range`,min:`1`,max:`80`,step:`1`,value:o,onChange:e=>s(Number(e.target.value))}),(0,T.jsx)(`strong`,{children:o})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Finite-difference epsilon`}),(0,T.jsxs)(`select`,{"aria-label":`Finite-difference epsilon`,value:n,onChange:e=>r(Number(e.target.value)),children:[(0,T.jsx)(`option`,{value:`0.01`,children:`1e-2`}),(0,T.jsx)(`option`,{value:`0.001`,children:`1e-3`}),(0,T.jsx)(`option`,{value:`0.0001`,children:`1e-4`}),(0,T.jsx)(`option`,{value:`0.00001`,children:`1e-5`}),(0,T.jsx)(`option`,{value:`0.000001`,children:`1e-6`}),(0,T.jsx)(`option`,{value:`1e-8`,children:`1e-8`})]})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Maximum relative gradient error`}),(0,T.jsx)(`strong`,{children:zc(c.maximumRelativeError)})]}),(0,T.jsx)(`button`,{type:`button`,onClick:v,children:`Reset optimization lab`})]})]})}var Gc=[1,2,0],Kc={inputWeight:2,recurrentWeight:.5,bias:-1},qc=.1,Jc=1e-6;function Yc(e){return Math.abs(e)<1e-12?0:e}function Xc(e=Gc,t=0,n=Kc,r=!0){if(e.length!==3||![...e,t,n.inputWeight,n.recurrentWeight,n.bias].every(Number.isFinite))throw Error(`NN09 V1 needs three finite inputs, state, and parameters.`);let i=t,a=e.map((e,t)=>{let a=Yc(n.inputWeight*e),o=r?Yc(n.recurrentWeight*i):0,s=Yc(a+o+n.bias),c=Yc(Math.max(0,s)),l={time:t,input:e,previousState:i,inputProduct:a,recurrentProduct:o,bias:n.bias,preactivation:s,state:c};return i=c,l});return{steps:a,states:a.map(e=>e.state),finalState:a[a.length-1].state}}function Zc(e,t,n,r){return .5*(Xc(e,t,n).finalState-r)**2}function Qc(e,t,n,r,i,a){let o={...r,[e]:r[e]+a},s={...r,[e]:r[e]-a};return(Zc(t,n,o,i)-Zc(t,n,s,i))/(2*a)}function $c(e=Gc,t=0,n=Kc,r=0,i=qc,a=Jc){if(![r,i,a].every(Number.isFinite)||a<=0)throw Error(`NN10 V1 needs a finite target and learning rate, plus a positive epsilon.`);let o=Xc(e,t,n),s=.5*(o.finalState-r)**2,c=0,l=[];for(let e=o.steps.length-1;e>=0;--e){let t=o.steps[e],i=e===o.steps.length-1?o.finalState-r:0,a=Yc(i+c),s=+(t.preactivation>0),u=Yc(a*s),d={inputWeight:Yc(u*t.input),recurrentWeight:Yc(u*t.previousState),bias:u},f=Yc(u*n.recurrentWeight);l.push({time:e,directStateGradient:i,futureStateGradient:c,stateGradient:a,reluDerivative:s,preactivationGradient:u,parameterContributions:d,previousStateGradient:f}),c=f}let u=l.reduce((e,t)=>({inputWeight:Yc(e.inputWeight+t.parameterContributions.inputWeight),recurrentWeight:Yc(e.recurrentWeight+t.parameterContributions.recurrentWeight),bias:Yc(e.bias+t.parameterContributions.bias),initialState:t.time===0?t.previousStateGradient:e.initialState}),{inputWeight:0,recurrentWeight:0,bias:0,initialState:0}),d={inputWeight:Qc(`inputWeight`,e,t,n,r,a),recurrentWeight:Qc(`recurrentWeight`,e,t,n,r,a),bias:Qc(`bias`,e,t,n,r,a)},f={inputWeight:Math.abs(u.inputWeight-d.inputWeight),recurrentWeight:Math.abs(u.recurrentWeight-d.recurrentWeight),bias:Math.abs(u.bias-d.bias)},p={inputWeight:Yc(n.inputWeight-i*u.inputWeight),recurrentWeight:Yc(n.recurrentWeight-i*u.recurrentWeight),bias:Yc(n.bias-i*u.bias)},m=Xc(e,t,p);return{forward:o,target:r,loss:s,backwardSteps:l,gradientTotals:u,numericalGradients:d,gradientErrors:f,maxGradientError:Math.max(...Object.values(f)),update:{learningRate:i,parameters:p,preactivations:m.steps.map(e=>e.preactivation),states:m.states,loss:.5*(m.finalState-r)**2}}}function R(e){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-6?e.toExponential(2):Number(e.toFixed(6)).toString()}function el({onShowForward:e,onShowGates:t}){let n=(0,l.useMemo)(()=>$c(),[]),[r,i]=(0,l.useState)(2),a=n.backwardSteps.find(e=>e.time===r),o=[...n.backwardSteps].reverse();return(0,T.jsxs)(`main`,{className:`workspace workspace--bptt`,children:[(0,T.jsxs)(`section`,{className:`bptt-stage`,"aria-label":`Backpropagation through time trace`,children:[(0,T.jsxs)(`div`,{className:`bptt-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN10 · sequence gradients`}),(0,T.jsx)(`h2`,{children:`Backpropagation-through-time microscope`}),(0,T.jsx)(`p`,{children:`Keep the three saved forward states, then reverse every arrow. Watch later evidence reach earlier cells and add into one shared gradient.`})]}),(0,T.jsxs)(`div`,{className:`bptt-loss-chip`,children:[(0,T.jsx)(`small`,{children:`final-state loss`}),(0,T.jsx)(`strong`,{children:R(n.loss)})]})]}),(0,T.jsxs)(`section`,{className:`bptt-panel`,"aria-label":`Forward states and backward gradient lane`,children:[(0,T.jsxs)(`div`,{className:`bptt-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Forward saved · backward reversed`}),(0,T.jsx)(`h2`,{children:`One chain, two directions`})]}),(0,T.jsxs)(`code`,{children:[`target = `,R(n.target)]})]}),(0,T.jsxs)(`div`,{className:`bptt-forward-lane`,"aria-label":`Saved forward states`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`initial`}),(0,T.jsx)(`strong`,{children:`h[-1] = 0`})]}),n.forward.steps.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`a[`,e.time,`] = `,R(e.preactivation)]}),(0,T.jsxs)(`strong`,{children:[`h[`,e.time,`] = `,R(e.state)]})]},e.time)),(0,T.jsxs)(`div`,{className:`bptt-forward-lane__loss`,children:[(0,T.jsx)(`small`,{children:`half-squared`}),(0,T.jsxs)(`strong`,{children:[`L = `,R(n.loss)]})]})]}),(0,T.jsxs)(`div`,{className:`bptt-direction-label`,children:[(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`←`}),`backward pass runs from t = 2 to t = 0`]}),(0,T.jsx)(`div`,{className:`bptt-backward-lane`,"aria-label":`Reverse-time gradient steps`,children:n.backwardSteps.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Select backward step ${e.time}`,"aria-pressed":r===e.time,className:r===e.time?`bptt-step bptt-step--active`:`bptt-step`,type:`button`,onClick:()=>i(e.time),children:[(0,T.jsxs)(`small`,{children:[`reverse t = `,e.time]}),(0,T.jsxs)(`strong`,{children:[`dL/dh = `,R(e.stateGradient)]}),(0,T.jsxs)(`span`,{children:[`dL/da = `,R(e.preactivationGradient)]})]},e.time))}),(0,T.jsxs)(`div`,{className:`bptt-arithmetic`,"aria-label":`Selected backward arithmetic`,children:[(0,T.jsxs)(`div`,{className:`bptt-arithmetic-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected · reverse step `,r]}),(0,T.jsx)(`h3`,{children:`Combine incoming gradient before differentiating`})]}),(0,T.jsxs)(`code`,{children:[`ReLU' = `,R(a.reluDerivative)]})]}),(0,T.jsxs)(`div`,{className:`bptt-equation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`direct loss`}),(0,T.jsx)(`strong`,{children:R(a.directStateGradient)})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`from future`}),(0,T.jsx)(`strong`,{children:R(a.futureStateGradient)})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`dL/dh[`,r,`]`]}),(0,T.jsx)(`strong`,{children:R(a.stateGradient)})]}),(0,T.jsx)(`span`,{children:`×`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`ReLU derivative`}),(0,T.jsx)(`strong`,{children:R(a.reluDerivative)})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{className:`bptt-equation__result`,children:[(0,T.jsxs)(`small`,{children:[`dL/da[`,r,`]`]}),(0,T.jsx)(`strong`,{children:R(a.preactivationGradient)})]})]}),(0,T.jsxs)(`div`,{className:`bptt-local-gradients`,children:[(0,T.jsxs)(`code`,{children:[`ΔW_x = `,R(a.parameterContributions.inputWeight)]}),(0,T.jsxs)(`code`,{children:[`ΔW_h = `,R(a.parameterContributions.recurrentWeight)]}),(0,T.jsxs)(`code`,{children:[`Δb = `,R(a.parameterContributions.bias)]}),(0,T.jsxs)(`code`,{children:[`to h[`,r-1,`] = `,R(a.previousStateGradient)]})]})]})]}),(0,T.jsxs)(`section`,{className:`bptt-panel`,"aria-label":`Shared gradient reduction`,children:[(0,T.jsxs)(`div`,{className:`bptt-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Three executions · one parameter set`}),(0,T.jsx)(`h2`,{children:`Shared gradients add; they do not overwrite`})]}),(0,T.jsx)(`strong`,{className:`bptt-pass`,children:`ACCUMULATE`})]}),(0,T.jsx)(`div`,{className:`bptt-table-wrap`,children:(0,T.jsxs)(`table`,{className:`bptt-table`,children:[(0,T.jsx)(`caption`,{children:`Per-time-step parameter contributions and their totals`}),(0,T.jsx)(`thead`,{children:(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`col`,children:`gradient`}),o.map(e=>(0,T.jsxs)(`th`,{scope:`col`,children:[`t = `,e.time]},e.time)),(0,T.jsx)(`th`,{scope:`col`,children:`total`})]})}),(0,T.jsxs)(`tbody`,{children:[(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`dL/dW_x`}),o.map(e=>(0,T.jsx)(`td`,{children:R(e.parameterContributions.inputWeight)},e.time)),(0,T.jsx)(`td`,{children:(0,T.jsx)(`strong`,{children:R(n.gradientTotals.inputWeight)})})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`dL/dW_h`}),o.map(e=>(0,T.jsx)(`td`,{children:R(e.parameterContributions.recurrentWeight)},e.time)),(0,T.jsx)(`td`,{children:(0,T.jsx)(`strong`,{children:R(n.gradientTotals.recurrentWeight)})})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`dL/db`}),o.map(e=>(0,T.jsx)(`td`,{children:R(e.parameterContributions.bias)},e.time)),(0,T.jsx)(`td`,{children:(0,T.jsx)(`strong`,{children:R(n.gradientTotals.bias)})})]})]})]})}),(0,T.jsxs)(`p`,{className:`bptt-initial-gradient`,children:[`The reverse chain continues into the explicit initial state:`,(0,T.jsxs)(`strong`,{children:[` dL/dh[-1] = `,R(n.gradientTotals.initialState)]})]})]}),(0,T.jsxs)(`section`,{className:`bptt-audit-grid`,"aria-label":`Gradient audit and update preview`,children:[(0,T.jsxs)(`div`,{className:`bptt-panel`,children:[(0,T.jsxs)(`div`,{className:`bptt-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Independent oracle`}),(0,T.jsx)(`h2`,{children:`Finite-difference gradient check`})]}),(0,T.jsx)(`strong`,{className:`bptt-pass`,children:`PASS`})]}),(0,T.jsx)(`div`,{className:`bptt-table-wrap`,children:(0,T.jsxs)(`table`,{className:`bptt-table`,children:[(0,T.jsx)(`caption`,{children:`Analytical and numerical gradient agreement`}),(0,T.jsx)(`thead`,{children:(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`col`,children:`parameter`}),(0,T.jsx)(`th`,{scope:`col`,children:`BPTT`}),(0,T.jsx)(`th`,{scope:`col`,children:`numerical`}),(0,T.jsx)(`th`,{scope:`col`,children:`error`})]})}),(0,T.jsx)(`tbody`,{children:[`inputWeight`,`recurrentWeight`,`bias`].map(e=>(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:e===`inputWeight`?`W_x`:e===`recurrentWeight`?`W_h`:`b`}),(0,T.jsx)(`td`,{children:R(n.gradientTotals[e])}),(0,T.jsx)(`td`,{children:R(n.numericalGradients[e])}),(0,T.jsx)(`td`,{children:R(n.gradientErrors[e])})]},e))})]})})]}),(0,T.jsxs)(`div`,{className:`bptt-panel bptt-update-panel`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One step · learning rate 0.1`}),(0,T.jsx)(`h2`,{children:`Move against the accumulated gradient`}),(0,T.jsxs)(`div`,{className:`bptt-loss-change`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`before loss`}),(0,T.jsx)(`strong`,{children:R(n.loss)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`after loss`}),(0,T.jsx)(`strong`,{children:R(n.update.loss)})]})]}),(0,T.jsxs)(`div`,{className:`bptt-parameter-update`,children:[(0,T.jsxs)(`code`,{children:[`W_x: `,R(Kc.inputWeight),` → `,R(n.update.parameters.inputWeight)]}),(0,T.jsxs)(`code`,{children:[`W_h: `,R(Kc.recurrentWeight),` → `,R(n.update.parameters.recurrentWeight)]}),(0,T.jsxs)(`code`,{children:[`b: `,R(Kc.bias),` → `,R(n.update.parameters.bias)]})]}),(0,T.jsxs)(`p`,{children:[`Updated states = [`,n.update.states.map(R).join(`, `),`]`]})]})]})]}),(0,T.jsxs)(`aside`,{className:`recurrent-controls bptt-controls`,"aria-label":`BPTT microscope controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Forward and backward belong together`}),(0,T.jsx)(`h2`,{children:`Reverse the unroll`}),(0,T.jsx)(`p`,{children:`Select a reverse-time cell. Its future gradient was produced by the cell immediately to its right in the forward graph.`}),(0,T.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:e,children:`Show forward unroll`}),(0,T.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:t,children:`Compare GRU and LSTM gates`}),(0,T.jsxs)(`div`,{className:`recurrent-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected reverse step`}),(0,T.jsxs)(`strong`,{children:[`t = `,r]}),(0,T.jsxs)(`span`,{children:[R(a.directStateGradient),` direct + `,R(a.futureStateGradient),` from the future.`]})]}),(0,T.jsxs)(`div`,{className:`recurrent-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Vectors use the same reverse walk with matrix products. GRUs and LSTMs add gates, while truncated BPTT limits how far this lane runs.`})]})]})]})}var tl=.8,nl=.8,rl=Math.log(3),il=Math.atanh(.6),al=il-.4;function ol(e){if(e>=0)return 1/(1+Math.exp(-e));let t=Math.exp(e);return t/(1+t)}function sl(e){return{preactivation:e,value:ol(e)}}function cl(e=1,t=tl,n=nl){if(![e,t,n].every(Number.isFinite))throw Error(`NN11 V1 needs finite scalar input and recurrent states.`);let r=sl(0),i=sl(-rl),a=r.value*t,o=0*e,s=a,c=o+s+al,l=Math.tanh(c),u=(1-i.value)*t,d=i.value*l,f=sl(0),p=sl(-rl),m=sl(rl),h={preactivation:il,value:Math.tanh(il)},g=f.value*n,_=p.value*h.value,v=g+_,y=Math.tanh(v);return{input:e,previousHidden:t,previousCell:n,gru:{resetGate:r,updateGate:i,candidate:{inputProduct:o,resetState:a,recurrentProduct:s,bias:al,preactivation:c,value:l},retainedState:u,candidateWrite:d,hiddenState:u+d},lstm:{forgetGate:f,inputGate:p,outputGate:m,candidate:h,retainedCell:g,candidateWrite:_,cellState:v,exposedCell:y,hiddenState:m.value*y}}}function ll(e,t,n,r=cl()){if(!Number.isFinite(n)||n<0||n>1)throw Error(`NN11 gate interventions must be between zero and one.`);if(e===`gru`){if(t!==`reset`&&t!==`update`)throw Error(`Gate ${t} does not belong to the GRU.`);let i=t===`reset`?n:r.gru.resetGate.value,a=t===`update`?n:r.gru.updateGate.value,o=Math.tanh(r.gru.candidate.inputProduct+i*r.previousHidden+r.gru.candidate.bias);return{model:e,gate:t,gateValue:n,candidate:o,cellState:null,hiddenState:(1-a)*r.previousHidden+a*o}}if(![`forget`,`input`,`output`].includes(t))throw Error(`Gate ${t} does not belong to the LSTM.`);let i=t===`forget`?n:r.lstm.forgetGate.value,a=t===`input`?n:r.lstm.inputGate.value,o=t===`output`?n:r.lstm.outputGate.value,s=i*r.previousCell+a*r.lstm.candidate.value;return{model:e,gate:t,gateValue:n,candidate:r.lstm.candidate.value,cellState:s,hiddenState:o*Math.tanh(s)}}function z(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function ul(e,t){return e===`gru`?t===`reset`?.5:.25:t===`forget`?.5:t===`input`?.25:.75}function dl({onShowBackward:e}){let t=(0,l.useMemo)(()=>cl(),[]),[n,r]=(0,l.useState)(`gru`),[i,a]=(0,l.useState)(`update`),[o,s]=(0,l.useState)(`canonical`),c=o===`canonical`?ul(n,i):o,u=ll(n,i,c,t),d=(e,t)=>{r(e),a(t),s(`canonical`)},f=n===`gru`&&i===`reset`?c:t.gru.resetGate.value,p=n===`gru`&&i===`update`?c:t.gru.updateGate.value,m=n===`gru`?u.candidate:t.gru.candidate.value,h=(1-p)*t.previousHidden,g=p*m,_=h+g,v=n===`lstm`&&i===`forget`?c:t.lstm.forgetGate.value,y=n===`lstm`&&i===`input`?c:t.lstm.inputGate.value,b=n===`lstm`&&i===`output`?c:t.lstm.outputGate.value,x=v*t.previousCell,ee=y*t.lstm.candidate.value,S=x+ee,C=b*Math.tanh(S);return(0,T.jsxs)(`main`,{className:`workspace workspace--gates`,children:[(0,T.jsxs)(`section`,{className:`gate-stage`,"aria-label":`GRU and LSTM gate comparison`,children:[(0,T.jsxs)(`div`,{className:`gate-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN11 · gated sequence memory`}),(0,T.jsx)(`h2`,{children:`GRU and LSTM gate comparator`}),(0,T.jsx)(`p`,{children:`Route the same previous memory and candidate through both cells. Change one gate while every other signal stays fixed.`})]}),(0,T.jsxs)(`div`,{className:`gate-input-chip`,children:[(0,T.jsx)(`small`,{children:`shared input`}),(0,T.jsx)(`strong`,{children:`x = 1 · h = 0.8`})]})]}),(0,T.jsxs)(`section`,{className:`gate-comparison-panel`,"aria-label":`Aligned gated memory lanes`,children:[(0,T.jsxs)(`div`,{className:`gate-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same evidence · different state design`}),(0,T.jsx)(`h2`,{children:`Follow what each gate lets through`})]}),(0,T.jsx)(`code`,{children:`candidate = 0.6`})]}),(0,T.jsxs)(`article`,{className:`gate-model-lane gate-model-lane--gru`,"aria-label":`GRU memory lane`,children:[(0,T.jsxs)(`div`,{className:`gate-model-label`,children:[(0,T.jsx)(`span`,{children:`GRU`}),(0,T.jsx)(`strong`,{children:`one stored and exposed state`})]}),(0,T.jsxs)(`div`,{className:`gate-flow`,children:[(0,T.jsxs)(`div`,{className:`gate-state-node`,children:[(0,T.jsx)(`small`,{children:`previous state`}),(0,T.jsxs)(`strong`,{children:[`h = `,z(t.previousHidden)]})]}),(0,T.jsxs)(`button`,{"aria-label":`Select GRU reset gate`,"aria-pressed":n===`gru`&&i===`reset`,className:n===`gru`&&i===`reset`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`gru`,`reset`),children:[(0,T.jsx)(`small`,{children:`reset r`}),(0,T.jsx)(`strong`,{children:z(f)}),(0,T.jsxs)(`span`,{children:[`candidate sees `,z(f*t.previousHidden)]})]}),(0,T.jsxs)(`div`,{className:`gate-candidate-node`,children:[(0,T.jsx)(`small`,{children:`candidate n`}),(0,T.jsx)(`strong`,{children:z(m)})]}),(0,T.jsxs)(`button`,{"aria-label":`Select GRU update gate`,"aria-pressed":n===`gru`&&i===`update`,className:n===`gru`&&i===`update`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`gru`,`update`),children:[(0,T.jsx)(`small`,{children:`update z`}),(0,T.jsx)(`strong`,{children:z(p)}),(0,T.jsx)(`span`,{children:`new share`})]}),(0,T.jsxs)(`div`,{className:`gate-result-node`,children:[(0,T.jsx)(`small`,{children:`next hidden`}),(0,T.jsxs)(`strong`,{children:[`h = `,z(_)]}),(0,T.jsxs)(`span`,{children:[z(h),` old + `,z(g),` new`]})]})]})]}),(0,T.jsxs)(`article`,{className:`gate-model-lane gate-model-lane--lstm`,"aria-label":`LSTM memory lane`,children:[(0,T.jsxs)(`div`,{className:`gate-model-label`,children:[(0,T.jsx)(`span`,{children:`LSTM`}),(0,T.jsx)(`strong`,{children:`private cell plus exposed hidden state`})]}),(0,T.jsxs)(`div`,{className:`gate-flow gate-flow--lstm`,children:[(0,T.jsxs)(`div`,{className:`gate-state-node`,children:[(0,T.jsx)(`small`,{children:`previous cell`}),(0,T.jsxs)(`strong`,{children:[`c = `,z(t.previousCell)]})]}),(0,T.jsxs)(`button`,{"aria-label":`Select LSTM forget gate`,"aria-pressed":n===`lstm`&&i===`forget`,className:n===`lstm`&&i===`forget`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`lstm`,`forget`),children:[(0,T.jsx)(`small`,{children:`forget f`}),(0,T.jsx)(`strong`,{children:z(v)}),(0,T.jsx)(`span`,{children:`old share`})]}),(0,T.jsxs)(`button`,{"aria-label":`Select LSTM input gate`,"aria-pressed":n===`lstm`&&i===`input`,className:n===`lstm`&&i===`input`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`lstm`,`input`),children:[(0,T.jsx)(`small`,{children:`input i`}),(0,T.jsx)(`strong`,{children:z(y)}),(0,T.jsx)(`span`,{children:`candidate share`})]}),(0,T.jsxs)(`div`,{className:`gate-cell-node`,children:[(0,T.jsx)(`small`,{children:`private cell`}),(0,T.jsxs)(`strong`,{children:[`c = `,z(S)]}),(0,T.jsxs)(`span`,{children:[z(x),` old + `,z(ee),` new`]})]}),(0,T.jsxs)(`button`,{"aria-label":`Select LSTM output gate`,"aria-pressed":n===`lstm`&&i===`output`,className:n===`lstm`&&i===`output`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`lstm`,`output`),children:[(0,T.jsx)(`small`,{children:`output o`}),(0,T.jsx)(`strong`,{children:z(b)}),(0,T.jsx)(`span`,{children:`visible share`})]}),(0,T.jsxs)(`div`,{className:`gate-result-node`,children:[(0,T.jsx)(`small`,{children:`next hidden`}),(0,T.jsxs)(`strong`,{children:[`h = `,z(C)]}),(0,T.jsx)(`span`,{children:`o × tanh(c)`})]})]})]})]}),(0,T.jsxs)(`section`,{className:`gate-comparison-panel`,"aria-label":`Gate responsibility comparison`,children:[(0,T.jsx)(`div`,{className:`gate-panel-heading`,children:(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Architecture, not acronym memorization`}),(0,T.jsx)(`h2`,{children:`Which signal does each gate control?`})]})}),(0,T.jsx)(`div`,{className:`gate-table-wrap`,children:(0,T.jsxs)(`table`,{className:`gate-table`,children:[(0,T.jsx)(`caption`,{children:`GRU and LSTM state-routing responsibilities`}),(0,T.jsx)(`thead`,{children:(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`col`,children:`Responsibility`}),(0,T.jsx)(`th`,{scope:`col`,children:`GRU`}),(0,T.jsx)(`th`,{scope:`col`,children:`LSTM`})]})}),(0,T.jsxs)(`tbody`,{children:[(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`Build candidate`}),(0,T.jsx)(`td`,{children:`reset gate`}),(0,T.jsx)(`td`,{children:`candidate tanh path`})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`Retain old memory`}),(0,T.jsx)(`td`,{rowSpan:2,children:`update gate mixes both`}),(0,T.jsx)(`td`,{children:`forget gate`})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`Write new memory`}),(0,T.jsx)(`td`,{children:`input gate`})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`Expose memory`}),(0,T.jsx)(`td`,{children:`same hidden state`}),(0,T.jsx)(`td`,{children:`output gate`})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`State buffers`}),(0,T.jsxs)(`td`,{children:[`h = `,z(_)]}),(0,T.jsxs)(`td`,{children:[`c = `,z(S),`, h = `,z(C)]})]})]})]})})]})]}),(0,T.jsxs)(`aside`,{className:`gate-controls`,"aria-label":`Gate intervention controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One controlled intervention`}),(0,T.jsxs)(`h2`,{children:[n.toUpperCase(),` `,i,` gate`]}),(0,T.jsx)(`p`,{children:`Keep every other gate fixed. Use the learned canonical value or force this one valve fully closed or open.`}),(0,T.jsxs)(`div`,{className:`gate-intervention-buttons`,"aria-label":`Selected gate value`,children:[(0,T.jsx)(`button`,{"aria-pressed":o===`canonical`,type:`button`,onClick:()=>s(`canonical`),children:`Canonical`}),(0,T.jsx)(`button`,{"aria-pressed":o===0,type:`button`,onClick:()=>s(0),children:`Force 0`}),(0,T.jsx)(`button`,{"aria-pressed":o===1,type:`button`,onClick:()=>s(1),children:`Force 1`})]}),(0,T.jsxs)(`div`,{className:`gate-selected-summary`,"aria-label":`Selected gate effect`,children:[(0,T.jsx)(`small`,{children:`selected value`}),(0,T.jsx)(`strong`,{children:z(c)}),(0,T.jsx)(`span`,{children:n===`gru`?`candidate ${z(m)} · next h ${z(_)}`:`next c ${z(S)} · visible h ${z(C)}`})]}),(0,T.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:e,children:`Return to BPTT gradients`}),(0,T.jsxs)(`div`,{className:`recurrent-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Vector cells pack each gate's affine projection into matrices. The scalar routing stays identical at every coordinate.`})]})]})]})}function fl(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function pl({onShowBackward:e}){let[t,n]=(0,l.useState)(0),[r,i]=(0,l.useState)(!0),a=(0,l.useMemo)(()=>Xc(),[]),o=(0,l.useMemo)(()=>Xc(Gc,0,Kc,!1),[]),s=r?a:o,c=s.steps[t];return(0,T.jsxs)(`main`,{className:`workspace workspace--recurrent`,children:[(0,T.jsxs)(`section`,{className:`recurrent-stage`,"aria-label":`Three-step recurrent state trace`,children:[(0,T.jsxs)(`div`,{className:`recurrent-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN09 · sequence networks`}),(0,T.jsx)(`h2`,{children:`Recurrent-state unroller`}),(0,T.jsx)(`p`,{children:`Run one scalar cell three times. Each result becomes part of the next input while one parameter set stays shared across time.`})]}),(0,T.jsx)(`div`,{className:`recurrent-sequence-chip`,children:`x = [1, 2, 0]`})]}),(0,T.jsxs)(`section`,{className:`recurrent-unroll-panel`,"aria-label":`Recurrent cell unroll`,children:[(0,T.jsxs)(`div`,{className:`recurrent-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One cell · three executions`}),(0,T.jsx)(`h2`,{children:`Follow the state from left to right`})]}),(0,T.jsxs)(`div`,{className:`recurrent-final-state`,children:[(0,T.jsx)(`small`,{children:`final state`}),(0,T.jsx)(`strong`,{children:fl(s.finalState)})]})]}),(0,T.jsxs)(`div`,{className:`shared-parameter-strip`,"aria-label":`Parameters shared by every time step`,children:[(0,T.jsx)(`span`,{children:`shared at t=0, 1, 2`}),(0,T.jsxs)(`code`,{children:[`Wₓ = `,fl(Kc.inputWeight)]}),(0,T.jsxs)(`code`,{children:[`Wₕ = `,fl(Kc.recurrentWeight)]}),(0,T.jsxs)(`code`,{children:[`b = `,fl(Kc.bias)]})]}),(0,T.jsxs)(`div`,{className:`recurrent-chain`,"aria-label":`Unrolled recurrent state chain`,children:[(0,T.jsxs)(`div`,{className:`recurrent-initial-node`,children:[(0,T.jsx)(`small`,{children:`initial`}),(0,T.jsx)(`strong`,{children:`h[-1]`}),(0,T.jsx)(`code`,{children:fl(0)})]}),s.steps.map(e=>(0,T.jsxs)(l.Fragment,{children:[(0,T.jsxs)(`div`,{className:r?`recurrent-connector`:`recurrent-connector recurrent-connector--disabled`,children:[(0,T.jsx)(`small`,{children:r?`carry h`:`cut`}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`→`})]}),(0,T.jsxs)(`button`,{"aria-label":`Select recurrent step ${e.time}`,"aria-pressed":t===e.time,className:t===e.time?`recurrent-cell recurrent-cell--active`:`recurrent-cell`,type:`button`,onClick:()=>n(e.time),children:[(0,T.jsxs)(`small`,{children:[`time `,e.time]}),(0,T.jsxs)(`span`,{children:[`x[`,e.time,`] = `,fl(e.input)]}),(0,T.jsxs)(`strong`,{children:[`h[`,e.time,`] = `,fl(e.state)]})]})]},e.time))]}),(0,T.jsxs)(`div`,{className:`recurrent-arithmetic`,"aria-label":`Selected recurrent arithmetic`,children:[(0,T.jsxs)(`div`,{className:`recurrent-arithmetic-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected · time `,t]}),(0,T.jsx)(`h3`,{children:`Open this cell`})]}),(0,T.jsxs)(`code`,{children:[`h[`,t-1,`] → h[`,t,`]`]})]}),(0,T.jsxs)(`div`,{className:`recurrent-equation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`new input`}),(0,T.jsxs)(`strong`,{children:[`2 × `,fl(c.input),` = `,fl(c.inputProduct)]})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{className:r?``:`equation-term--disabled`,children:[(0,T.jsx)(`small`,{children:`carried state`}),(0,T.jsxs)(`strong`,{children:[`0.5 × `,fl(c.previousState),` = `,fl(c.recurrentProduct)]})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias`}),(0,T.jsx)(`strong`,{children:fl(c.bias)})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`preactivation`}),(0,T.jsx)(`strong`,{children:fl(c.preactivation)})]}),(0,T.jsx)(`span`,{children:`→`}),(0,T.jsxs)(`div`,{className:`recurrent-equation__state`,children:[(0,T.jsx)(`small`,{children:`ReLU state`}),(0,T.jsx)(`strong`,{children:fl(c.state)})]})]})]})]}),(0,T.jsxs)(`section`,{className:`memory-ablation-panel`,"aria-label":`Recurrent memory ablation`,children:[(0,T.jsxs)(`div`,{className:`recurrent-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same inputs · memory removed`}),(0,T.jsx)(`h2`,{children:`What came through the recurrent link?`})]}),(0,T.jsx)(`p`,{children:`The final zero input remembers earlier steps only when the link is present.`})]}),(0,T.jsx)(`div`,{className:`recurrent-table-wrap`,children:(0,T.jsxs)(`table`,{className:`recurrent-table`,children:[(0,T.jsx)(`caption`,{children:`State comparison with and without recurrence`}),(0,T.jsx)(`thead`,{children:(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`col`,children:`time`}),(0,T.jsx)(`th`,{scope:`col`,children:`input`}),(0,T.jsx)(`th`,{scope:`col`,children:`with memory`}),(0,T.jsx)(`th`,{scope:`col`,children:`without memory`}),(0,T.jsx)(`th`,{scope:`col`,children:`difference`})]})}),(0,T.jsx)(`tbody`,{children:a.steps.map((e,n)=>{let r=o.states[n];return(0,T.jsxs)(`tr`,{className:t===n?`recurrent-table-row--active`:``,children:[(0,T.jsx)(`th`,{scope:`row`,children:n}),(0,T.jsx)(`td`,{children:fl(e.input)}),(0,T.jsx)(`td`,{children:fl(e.state)}),(0,T.jsx)(`td`,{children:fl(r)}),(0,T.jsx)(`td`,{children:fl(e.state-r)})]},n)})})]})})]})]}),(0,T.jsxs)(`aside`,{className:`recurrent-controls`,"aria-label":`Recurrent unroll controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One honest experiment`}),(0,T.jsx)(`h2`,{children:`Memory control`}),(0,T.jsx)(`p`,{children:`Select a time-step cell, then cut the recurrent link without changing its inputs, weights, or bias.`}),(0,T.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:e,children:`Trace backward gradients`}),(0,T.jsxs)(`label`,{className:`recurrent-memory-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:r,onChange:e=>i(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Carry the previous state`}),(0,T.jsx)(`small`,{children:`Use Wₕ × h[t - 1] at every step.`})]})]}),(0,T.jsxs)(`div`,{className:`recurrent-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected time`}),(0,T.jsxs)(`strong`,{children:[`t = `,t]}),(0,T.jsx)(`span`,{children:r?`${fl(c.recurrentProduct)} enters through memory.`:`The recurrent contribution is forced to zero.`})]}),(0,T.jsxs)(`div`,{className:`recurrent-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`Vector states repeat this same pattern across several coordinates. Backpropagation will reverse the unrolled arrows and add gradient contributions into the shared parameters.`})]})]})]})}function ml(){let[e,t]=(0,l.useState)(`forward`);return e===`backward`?(0,T.jsx)(el,{onShowForward:()=>t(`forward`),onShowGates:()=>t(`gates`)}):e===`gates`?(0,T.jsx)(dl,{onShowBackward:()=>t(`backward`)}):(0,T.jsx)(pl,{onShowBackward:()=>t(`backward`)})}var hl=[2,-1],gl={encoder:{weights:[.5,-.25],bias:0},decoder:{weights:[1.2,-.8],bias:[.1,-.2]}},_l=.1,vl=[`encoder.weights[0]`,`encoder.weights[1]`,`encoder.bias`,`decoder.weights[0]`,`decoder.weights[1]`,`decoder.bias[0]`,`decoder.bias[1]`];function yl(e){return Math.abs(e)<1e-12?0:e}function bl(e,t){return e.length===t&&e.every(Number.isFinite)}function xl(e){return{encoder:{weights:[...e.encoder.weights],bias:e.encoder.bias},decoder:{weights:[...e.decoder.weights],bias:[...e.decoder.bias]}}}function Sl(e,t){let n=e.map((e,n)=>yl(e*t.encoder.weights[n])),r=yl(n.reduce((e,t)=>e+t,0)+t.encoder.bias),i=t.decoder.weights.map(e=>yl(r*e)),a=i.map((e,n)=>yl(e+t.decoder.bias[n])),o=a.map((t,n)=>yl(t-e[n])),s=o.map(e=>e*e);return{encoderProducts:n,bottleneck:r,decoderProducts:i,reconstruction:a,errors:o,squaredErrors:s,loss:s.reduce((e,t)=>e+t,0)/2}}function Cl(e){return[...e.encoder.weights,e.encoder.bias,...e.decoder.weights,...e.decoder.bias]}function wl(e){return{encoder:{weights:e.slice(0,2),bias:e[2]},decoder:{weights:e.slice(3,5),bias:e.slice(5,7)}}}function Tl(e=_l,t=hl,n=gl){if(!Number.isFinite(e)||e<=0||!bl(t,2)||!bl(n.encoder.weights,2)||!Number.isFinite(n.encoder.bias)||!bl(n.decoder.weights,2)||!bl(n.decoder.bias,2))throw Error(`NN16 V1 needs a two-number input, 2 -> 1 -> 2 finite parameters, and a positive learning rate.`);let r=xl(n),i=Sl(t,r),a=[...i.errors],o=a.map(e=>yl(e*i.bottleneck)),s=[...a],c=a.map((e,t)=>yl(e*r.decoder.weights[t])),l=yl(c.reduce((e,t)=>e+t,0)),u=t.map(e=>yl(l*e)),d=l,f={reconstructionGradients:a,decoderWeightGradients:o,decoderBiasGradients:s,bottleneckGradientContributions:c,bottleneckGradient:l,encoderWeightGradients:u,encoderBiasGradient:d},p=[...u,d,...o,...s],m=Cl(r),h=1e-6,g=m.map((e,n)=>{let r=[...m],i=[...m];return r[n]+=h,i[n]-=h,(Sl(t,wl(r)).loss-Sl(t,wl(i)).loss)/(2*h)}),_=Math.max(...p.map((e,t)=>Math.abs(e-g[t]))),v={encoder:{weights:r.encoder.weights.map((t,n)=>t-e*u[n]),bias:r.encoder.bias-e*d},decoder:{weights:r.decoder.weights.map((t,n)=>t-e*o[n]),bias:r.decoder.bias.map((t,n)=>t-e*s[n])}},y=Sl(t,v);return{input:[...t],learningRate:e,parameters:r,forward:i,backward:f,gradientCheck:{epsilon:h,parameterOrder:[...vl],analytical:p,numerical:g,maxAbsoluteError:_},updatedParameters:v,postUpdate:y}}function B(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function El(e){return`[${e.map(B).join(`, `)}]`}function Dl(){let e=(0,l.useMemo)(()=>Tl(),[]),[t,n]=(0,l.useState)(0),[r,i]=(0,l.useState)(!1),a=r?e.postUpdate:e.forward,o=r?e.updatedParameters:e.parameters;return(0,T.jsxs)(`main`,{className:`workspace workspace--autoencoder`,children:[(0,T.jsxs)(`section`,{className:`autoencoder-stage`,"aria-label":`Two-number autoencoder bottleneck trace`,children:[(0,T.jsxs)(`div`,{className:`autoencoder-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN16 - representation through constraint`}),(0,T.jsx)(`h2`,{children:`Two numbers through one bottleneck`}),(0,T.jsx)(`p`,{children:`Compress a two-coordinate input into one scalar, reconstruct both coordinates from that shared value, and follow both errors back through one audited SGD step.`})]}),(0,T.jsx)(`div`,{className:`autoencoder-chip`,children:`2 -> 1 -> 2`})]}),(0,T.jsxs)(`section`,{className:`autoencoder-network-panel`,"aria-label":`Autoencoder encode and decode path`,children:[(0,T.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`The decoder never sees the original pair`}),(0,T.jsx)(`h2`,{children:`One scalar must serve two reconstructions`})]}),(0,T.jsx)(`code`,{children:r?`after one SGD step`:`saved forward pass`})]}),(0,T.jsxs)(`div`,{className:`autoencoder-network`,children:[(0,T.jsxs)(`div`,{className:`autoencoder-input-stack`,children:[(0,T.jsx)(`small`,{children:`input is also target`}),e.input.map((e,t)=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`span`,{children:[`x`,t]}),(0,T.jsx)(`strong`,{children:B(e)})]},t))]}),(0,T.jsx)(`span`,{className:`autoencoder-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`autoencoder-encoder-stack`,children:[(0,T.jsx)(`small`,{children:`encoder products`}),a.encoderProducts.map((t,n)=>(0,T.jsxs)(`code`,{children:[B(e.input[n]),` x `,B(o.encoder.weights[n]),` = `,B(t)]},n)),(0,T.jsxs)(`code`,{children:[`+ bias `,B(o.encoder.bias)]})]}),(0,T.jsx)(`span`,{className:`autoencoder-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`autoencoder-bottleneck`,children:[(0,T.jsx)(`small`,{children:`bottleneck z`}),(0,T.jsx)(`strong`,{children:B(a.bottleneck)}),(0,T.jsx)(`span`,{children:`one saved number`})]}),(0,T.jsx)(`span`,{className:`autoencoder-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`autoencoder-output-stack`,children:[(0,T.jsx)(`small`,{children:`decoder reconstructions`}),a.reconstruction.map((r,i)=>(0,T.jsxs)(`button`,{"aria-label":`Select reconstruction ${i}`,"aria-pressed":t===i,type:`button`,onClick:()=>n(i),children:[(0,T.jsxs)(`span`,{children:[`x_hat`,i]}),(0,T.jsx)(`strong`,{children:B(r)}),(0,T.jsxs)(`small`,{children:[`target `,B(e.input[i])]})]},i))]})]})]}),(0,T.jsxs)(`section`,{className:`autoencoder-reconstruction-panel`,"aria-label":`Selected autoencoder reconstruction ${t}`,children:[(0,T.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected - reconstruction `,t]}),(0,T.jsx)(`h2`,{children:`Decode and measure one coordinate`})]}),(0,T.jsxs)(`div`,{className:`autoencoder-loss-badge`,children:[(0,T.jsx)(`small`,{children:`total mean loss`}),(0,T.jsx)(`strong`,{children:B(a.loss)})]})]}),(0,T.jsxs)(`div`,{className:`autoencoder-reconstruction-flow`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`shared bottleneck`}),(0,T.jsxs)(`code`,{children:[`z = `,B(a.bottleneck)]})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`x`}),(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`decoder weight `,t]}),(0,T.jsx)(`code`,{children:B(o.decoder.weights[t])})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`decoder bias `,t]}),(0,T.jsx)(`code`,{children:B(o.decoder.bias[t])})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,T.jsxs)(`div`,{className:`autoencoder-reconstruction-result`,children:[(0,T.jsx)(`small`,{children:`reconstruction`}),(0,T.jsx)(`strong`,{children:B(a.reconstruction[t])})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`-`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`input target`}),(0,T.jsx)(`code`,{children:B(e.input[t])})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,T.jsxs)(`div`,{className:`autoencoder-error-result`,children:[(0,T.jsx)(`small`,{children:`error / loss gradient`}),(0,T.jsx)(`strong`,{children:B(a.errors[t])}),(0,T.jsxs)(`code`,{children:[`squared `,B(a.squaredErrors[t])]})]})]})]}),(0,T.jsxs)(`section`,{className:`autoencoder-backward-panel`,"aria-label":`Autoencoder bottleneck gradient trace`,children:[(0,T.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Two decoder branches meet at z`}),(0,T.jsx)(`h2`,{children:`Reconstruction error flows back through compression`})]}),(0,T.jsx)(`code`,{children:`dL/dz = sum of both routes`})]}),(0,T.jsxs)(`div`,{className:`autoencoder-branch-gradients`,children:[e.backward.bottleneckGradientContributions.map((r,i)=>(0,T.jsxs)(`button`,{"aria-label":`Select reconstruction gradient ${i}`,"aria-pressed":t===i,type:`button`,onClick:()=>n(i),children:[(0,T.jsxs)(`small`,{children:[`output `,i,` route`]}),(0,T.jsxs)(`code`,{children:[B(e.backward.reconstructionGradients[i]),` x `,B(e.parameters.decoder.weights[i])]}),(0,T.jsx)(`strong`,{children:B(r)})]},i)),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`sum`}),(0,T.jsxs)(`div`,{className:`autoencoder-bottleneck-gradient`,children:[(0,T.jsx)(`small`,{children:`bottleneck gradient`}),(0,T.jsx)(`strong`,{children:B(e.backward.bottleneckGradient)})]})]}),(0,T.jsxs)(`div`,{className:`autoencoder-gradient-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder weight gradients`}),(0,T.jsx)(`code`,{children:El(e.backward.decoderWeightGradients)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder bias gradients`}),(0,T.jsx)(`code`,{children:El(e.backward.decoderBiasGradients)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`encoder weight gradients`}),(0,T.jsx)(`code`,{children:El(e.backward.encoderWeightGradients)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`encoder bias gradient`}),(0,T.jsx)(`code`,{children:B(e.backward.encoderBiasGradient)})]})]})]}),(0,T.jsxs)(`section`,{className:`autoencoder-update-panel`,"aria-label":`Autoencoder SGD update and gradient audit`,children:[(0,T.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`All seven parameters move together`}),(0,T.jsx)(`h2`,{children:`Audit, update, rerun`})]}),(0,T.jsxs)(`code`,{children:[`parameter - `,e.learningRate,` x gradient`]})]}),(0,T.jsxs)(`div`,{className:`autoencoder-parameter-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`encoder before`}),(0,T.jsxs)(`code`,{children:[`w `,El(e.parameters.encoder.weights)]}),(0,T.jsxs)(`code`,{children:[`b `,B(e.parameters.encoder.bias)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`encoder after`}),(0,T.jsxs)(`code`,{children:[`w `,El(e.updatedParameters.encoder.weights)]}),(0,T.jsxs)(`code`,{children:[`b `,B(e.updatedParameters.encoder.bias)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder before`}),(0,T.jsxs)(`code`,{children:[`w `,El(e.parameters.decoder.weights)]}),(0,T.jsxs)(`code`,{children:[`b `,El(e.parameters.decoder.bias)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder after`}),(0,T.jsxs)(`code`,{children:[`w `,El(e.updatedParameters.decoder.weights)]}),(0,T.jsxs)(`code`,{children:[`b `,El(e.updatedParameters.decoder.bias)]})]})]}),(0,T.jsxs)(`div`,{className:`autoencoder-gradient-audit`,children:[(0,T.jsx)(`span`,{children:`Central finite differences - 7 parameters`}),(0,T.jsxs)(`code`,{children:[`epsilon = `,e.gradientCheck.epsilon]}),(0,T.jsxs)(`strong`,{children:[`max error `,e.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,T.jsxs)(`div`,{className:`autoencoder-loss-drop`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`loss before`}),(0,T.jsx)(`strong`,{children:B(e.forward.loss)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`loss after`}),(0,T.jsx)(`strong`,{children:B(e.postUpdate.loss)})]}),(0,T.jsx)(`p`,{children:`One reconstruction improves sharply; the shared mean objective falls.`})]})]})]}),(0,T.jsxs)(`aside`,{className:`autoencoder-controls`,"aria-label":`Autoencoder trace controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Open one decoder branch`}),(0,T.jsx)(`h2`,{children:`Bottleneck controls`}),(0,T.jsx)(`p`,{children:`Both outputs stay visible. Selection follows one reconstruction's arithmetic and gradient route without disconnecting the shared scalar.`}),(0,T.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Autoencoder reconstruction selection`,children:[0,1].map(e=>(0,T.jsxs)(`button`,{"aria-pressed":t===e,type:`button`,onClick:()=>n(e),children:[`output `,e]},e))}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:r,onChange:e=>i(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Use updated parameters`}),(0,T.jsx)(`small`,{children:`Rerun encode, decode, and loss after one SGD step.`})]})]}),(0,T.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected reconstruction`}),(0,T.jsxs)(`strong`,{children:[`x_hat`,t]}),(0,T.jsxs)(`span`,{children:[B(a.reconstruction[t]),` versus target `,B(e.input[t])]})]}),(0,T.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,T.jsx)(`span`,{children:`What is actually compressed?`}),(0,T.jsx)(`p`,{children:`The decoder receives z only. It cannot inspect either original coordinate while rebuilding the pair.`})]}),(0,T.jsxs)(`div`,{className:`attention-next-note`,children:[(0,T.jsx)(`span`,{children:`Keep the claim small`}),(0,T.jsx)(`p`,{children:`One example explains the mechanics. A useful representation needs many examples to reveal a shared lower-dimensional pattern.`})]})]})]})}var Ol=-.5,kl=[{t:1,beta:.36,normalizedT:.5},{t:2,beta:.4375,normalizedT:1}],Al={sampleWeight:0,timestepWeight:0,bias:0},jl=.5,Ml=[`denoiser.sample_weight`,`denoiser.timestep_weight`,`denoiser.bias`];function Nl(e){return{...e}}function Pl(e,t,n){let r=1;return n.map(n=>{let i=1-n.beta;r*=i;let a=Math.sqrt(r),o=Math.sqrt(1-r),s=a*e,c=o*t;return{...n,alpha:i,alphaBar:r,signalScale:a,noiseScale:o,signalContribution:s,noiseContribution:c,noisySample:s+c}})}function Fl(e,t,n){let r=e.map(e=>{let r=n.sampleWeight*e.noisySample+n.timestepWeight*e.normalizedT+n.bias,i=r-t;return{t:e.t,noisySample:e.noisySample,normalizedT:e.normalizedT,predictedNoise:r,targetNoise:t,error:i,loss:.5*i*i}});return{rows:r,meanLoss:r.reduce((e,t)=>e+t.loss,0)/r.length}}function Il(e,t){let n=e[e.length-1].noisySample;return[...e].reverse().map(e=>{let r=t.sampleWeight*n+t.timestepWeight*e.normalizedT+t.bias,i=e.beta/e.noiseScale,a=i*r,o=n-a,s=Math.sqrt(e.alpha),c=o/s,l={t:e.t,inputSample:n,normalizedT:e.normalizedT,predictedNoise:r,noiseCoefficient:i,scaledNoiseCorrection:a,correctedSample:o,alphaScale:s,outputMean:c};return n=c,l})}function Ll(e=1,t=Ol,n=jl,r=Al,i=kl){if(![e,t,n,r.sampleWeight,r.timestepWeight,r.bias,...i.flatMap(e=>[e.t,e.beta,e.normalizedT])].every(Number.isFinite)||n<=0||i.length<2||i.some((e,t)=>!Number.isInteger(e.t)||e.t!==t+1||e.beta<=0||e.beta>=1||e.normalizedT<=(i[t-1]?.normalizedT??0)||e.normalizedT>1)||Math.abs(i[i.length-1].normalizedT-1)>1e-12)throw Error(`NN19 V1 needs finite scalars, a positive learning rate, and consecutive increasing diffusion steps ending at normalized time 1.`);let a=Nl(r),o=i.map(e=>({...e})),s=Pl(e,t,o),c=Fl(s,t,a),l=c.rows.length,u=c.rows.map(e=>{let t=e.error/l;return{t:e.t,predictionGradient:t,sampleWeightContribution:t*e.noisySample,timestepWeightContribution:t*e.normalizedT,biasContribution:t}}),d=u.reduce((e,t)=>e+t.sampleWeightContribution,0),f=u.reduce((e,t)=>e+t.timestepWeightContribution,0),p=u.reduce((e,t)=>e+t.biasContribution,0),m=[d,f,p],h=[a.sampleWeight,a.timestepWeight,a.bias],g=1e-6,_=h.map((e,n)=>{let r=[...h],i=[...h];r[n]+=g,i[n]-=g;let a=e=>Fl(s,t,{sampleWeight:e[0],timestepWeight:e[1],bias:e[2]}).meanLoss;return(a(r)-a(i))/(2*g)}),v=Math.max(...m.map((e,t)=>Math.abs(e-_[t]))),y={sampleWeight:a.sampleWeight-n*d,timestepWeight:a.timestepWeight-n*f,bias:a.bias-n*p},b=Fl(s,t,y),x=Il(s,y),ee=x[x.length-1].outputMean;return{cleanSample:e,savedNoise:t,learningRate:n,schedule:o,denoiser:a,forwardSteps:s,initialDenoising:c.rows,initialMeanLoss:c.meanLoss,backward:{perStep:u,sampleWeightGradient:d,timestepWeightGradient:f,biasGradient:p},gradientCheck:{epsilon:g,parameterOrder:[...Ml],analytical:m,numerical:_,maxAbsoluteError:v},updatedDenoiser:y,postUpdateDenoising:b.rows,postUpdateMeanLoss:b.meanLoss,reverseSteps:x,finalReconstruction:ee,finalAbsoluteError:Math.abs(ee-e)}}var Rl=[{value:`clean`,shortLabel:`0. Data`,label:`Clean sample`},{value:`forward1`,shortLabel:`1. Forward`,label:`Noise level 1`},{value:`forward2`,shortLabel:`2. Forward`,label:`Noise level 2`},{value:`learn`,shortLabel:`3. Learn`,label:`Predict saved noise`},{value:`reverse2`,shortLabel:`4. Reverse`,label:`Denoise step 2`},{value:`reverse1`,shortLabel:`5. Reverse`,label:`Denoise step 1`}];function V(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function zl(){let[e,t]=(0,l.useState)(`clean`),n=(0,l.useMemo)(()=>Ll(),[]),r=Rl.findIndex(t=>t.value===e),i=r>=3,a=i?n.postUpdateDenoising:n.initialDenoising,o=i?n.postUpdateMeanLoss:n.initialMeanLoss,s=i?n.updatedDenoiser:n.denoiser,c=r>=4,u=r>=5,d=e===`clean`?n.cleanSample:e===`forward1`?n.forwardSteps[0].noisySample:e===`reverse2`?n.reverseSteps[0].outputMean:e===`reverse1`?n.finalReconstruction:n.forwardSteps[1].noisySample;return(0,T.jsxs)(`main`,{className:`workspace workspace--diffusion`,children:[(0,T.jsxs)(`section`,{className:`diffusion-stage`,"aria-label":`One-dimensional diffusion trace`,children:[(0,T.jsxs)(`div`,{className:`diffusion-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN19 - add known noise, then learn to remove it`}),(0,T.jsx)(`h2`,{children:`One clean number through a diffusion round trip`}),(0,T.jsx)(`p`,{children:`Trade signal for one saved noise value at two known levels, train a timestep-aware predictor, and follow its deterministic reverse mean back toward the data.`})]}),(0,T.jsx)(`div`,{className:`diffusion-chip`,children:`x0 -> x1 -> x2 -> mean1 -> mean0`})]}),(0,T.jsxs)(`section`,{className:`diffusion-forward-panel`,"aria-label":`Diffusion forward noise schedule`,children:[(0,T.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One epsilon, two comparable noise levels`}),(0,T.jsx)(`h2`,{children:`Signal shrinks while noise grows`})]}),(0,T.jsxs)(`code`,{children:[`saved epsilon = `,V(n.savedNoise)]})]}),(0,T.jsxs)(`div`,{className:`diffusion-forward-lane`,children:[(0,T.jsxs)(`div`,{className:e===`clean`?`diffusion-state diffusion-state--active`:`diffusion-state`,children:[(0,T.jsx)(`small`,{children:`clean data`}),(0,T.jsxs)(`strong`,{children:[`x0 = `,V(n.cleanSample)]}),(0,T.jsx)(`span`,{children:`100% signal`})]}),n.forwardSteps.map((t,r)=>(0,T.jsxs)(`div`,{className:`diffusion-forward-hop`,children:[(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+ noise`}),(0,T.jsxs)(`div`,{className:e===`forward${t.t}`?`diffusion-state diffusion-state--active diffusion-state--noisy`:`diffusion-state diffusion-state--noisy`,children:[(0,T.jsxs)(`small`,{children:[`noise level `,t.t]}),(0,T.jsxs)(`code`,{children:[V(t.signalScale),` x `,V(n.cleanSample),` + `,V(t.noiseScale),` x (`,V(n.savedNoise),`)`]}),(0,T.jsxs)(`strong`,{children:[`x`,t.t,` = `,V(t.noisySample)]}),(0,T.jsxs)(`span`,{children:[`alpha_bar = `,V(t.alphaBar)]})]})]},t.t))]}),(0,T.jsx)(`div`,{className:`diffusion-coefficient-grid`,children:n.forwardSteps.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`level `,e.t,` contributions`]}),(0,T.jsxs)(`code`,{children:[`signal `,V(e.signalContribution)]}),(0,T.jsxs)(`code`,{children:[`noise `,V(e.noiseContribution)]}),(0,T.jsxs)(`strong`,{children:[V(e.signalContribution),` + `,V(e.noiseContribution),` = `,V(e.noisySample)]})]},e.t))}),(0,T.jsx)(`p`,{className:`diffusion-forward-note`,children:`Each row samples directly from x0 with the same saved epsilon. That makes coefficient changes comparable; it is not one Markov noise path.`})]}),(0,T.jsxs)(`section`,{className:`diffusion-predict-panel`,"aria-label":`Diffusion noise prediction objective`,children:[(0,T.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`The model predicts corruption, not x0 directly`}),(0,T.jsx)(`h2`,{children:`Condition the denoiser on sample and timestep`})]}),(0,T.jsxs)(`div`,{className:`diffusion-loss-badge`,children:[(0,T.jsx)(`small`,{children:i?`mean loss after SGD`:`initial mean loss`}),(0,T.jsx)(`strong`,{children:V(o)})]})]}),(0,T.jsx)(`div`,{className:`diffusion-equation`,children:(0,T.jsxs)(`code`,{children:[`epsilon_hat = `,V(s.sampleWeight),` x x_t + `,V(s.timestepWeight),` x normalized_t + `,V(s.bias)]})}),(0,T.jsx)(`div`,{className:`diffusion-prediction-grid`,children:a.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`level `,e.t,`, normalized t = `,V(e.normalizedT)]}),(0,T.jsxs)(`code`,{children:[`input x`,e.t,` = `,V(e.noisySample)]}),(0,T.jsxs)(`strong`,{children:[`predicted `,V(e.predictedNoise)]}),(0,T.jsxs)(`span`,{children:[`target `,V(e.targetNoise)]}),(0,T.jsxs)(`span`,{children:[`half-squared loss `,V(e.loss)]})]},e.t))})]}),(0,T.jsxs)(`section`,{className:`diffusion-gradient-panel`,"aria-label":`Diffusion denoiser gradient and update`,children:[(0,T.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Both timesteps train one shared denoiser`}),(0,T.jsx)(`h2`,{children:`Add row contributions, audit, then update`})]}),(0,T.jsxs)(`code`,{children:[`parameter - `,V(n.learningRate),` x gradient`]})]}),(0,T.jsx)(`div`,{className:`diffusion-gradient-rows`,children:n.backward.perStep.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`level `,e.t]}),(0,T.jsxs)(`code`,{children:[`dL / d prediction = `,V(e.predictionGradient)]}),(0,T.jsxs)(`span`,{children:[`sample-w route `,V(e.sampleWeightContribution)]}),(0,T.jsxs)(`span`,{children:[`time-w route `,V(e.timestepWeightContribution)]}),(0,T.jsxs)(`span`,{children:[`bias route `,V(e.biasContribution)]})]},e.t))}),(0,T.jsxs)(`div`,{className:`diffusion-gradient-sum`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`sample weight gradient`}),(0,T.jsx)(`strong`,{children:V(n.backward.sampleWeightGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`timestep weight gradient`}),(0,T.jsx)(`strong`,{children:V(n.backward.timestepWeightGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias gradient`}),(0,T.jsx)(`strong`,{children:V(n.backward.biasGradient)})]})]}),(0,T.jsxs)(`div`,{className:`diffusion-update-row`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`parameters before -> after`}),(0,T.jsxs)(`code`,{children:[`sample w `,V(n.denoiser.sampleWeight),` -> `,V(n.updatedDenoiser.sampleWeight)]}),(0,T.jsxs)(`code`,{children:[`time w `,V(n.denoiser.timestepWeight),` -> `,V(n.updatedDenoiser.timestepWeight)]}),(0,T.jsxs)(`code`,{children:[`bias `,V(n.denoiser.bias),` -> `,V(n.updatedDenoiser.bias)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`central finite-difference audit`}),(0,T.jsx)(`strong`,{children:`3 parameters`}),(0,T.jsxs)(`code`,{children:[`max error `,n.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,T.jsxs)(`div`,{className:`diffusion-loss-drop`,children:[(0,T.jsx)(`small`,{children:`same two rows rerun`}),(0,T.jsxs)(`strong`,{children:[V(n.initialMeanLoss),` -> `,V(n.postUpdateMeanLoss)]}),(0,T.jsx)(`span`,{children:`noise prediction improves`})]})]})]}),(0,T.jsxs)(`section`,{className:`diffusion-reverse-panel`,"aria-label":`Diffusion deterministic reverse mean path`,children:[(0,T.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Subtract predicted noise one level at a time`}),(0,T.jsx)(`h2`,{children:`Run the updated model backward`})]}),(0,T.jsx)(`code`,{children:`no fresh reverse noise in this audit`})]}),(0,T.jsxs)(`div`,{className:`diffusion-reverse-lane`,children:[(0,T.jsxs)(`div`,{className:`diffusion-state diffusion-state--noisy`,children:[(0,T.jsx)(`small`,{children:`start at noisiest sample`}),(0,T.jsxs)(`strong`,{children:[`x2 = `,V(n.forwardSteps[1].noisySample)]})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:e===`reverse2`?`diffusion-reverse-step diffusion-reverse-step--active`:`diffusion-reverse-step`,children:[(0,T.jsx)(`small`,{children:`reverse t = 2`}),c?(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`code`,{children:[V(n.reverseSteps[0].inputSample),` - (`,V(n.reverseSteps[0].scaledNoiseCorrection),`)`]}),(0,T.jsxs)(`strong`,{children:[`mean1 = `,V(n.reverseSteps[0].outputMean)]}),(0,T.jsxs)(`span`,{children:[`predicted noise `,V(n.reverseSteps[0].predictedNoise)]})]}):(0,T.jsx)(`strong`,{children:`?`})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:e===`reverse1`?`diffusion-reverse-step diffusion-reverse-step--active`:`diffusion-reverse-step`,children:[(0,T.jsx)(`small`,{children:`reverse t = 1`}),u?(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`code`,{children:[V(n.reverseSteps[1].inputSample),` - (`,V(n.reverseSteps[1].scaledNoiseCorrection),`)`]}),(0,T.jsxs)(`strong`,{children:[`mean0 = `,V(n.finalReconstruction)]}),(0,T.jsxs)(`span`,{children:[`predicted noise `,V(n.reverseSteps[1].predictedNoise)]})]}):(0,T.jsx)(`strong`,{children:`?`})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`diffusion-final-state`,children:[(0,T.jsx)(`small`,{children:`reconstructed clean sample`}),(0,T.jsx)(`strong`,{children:u?V(n.finalReconstruction):`?`}),(0,T.jsx)(`span`,{children:u?`absolute error ${V(n.finalAbsoluteError)}`:`finish both reverse means`})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`diffusion-controls`,"aria-label":`Diffusion phase controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Round-trip schedule`}),(0,T.jsx)(`h2`,{children:`Advance the process`}),(0,T.jsx)(`p`,{children:`Forward levels share saved noise. Reverse levels reuse the learned denoiser but feed each generated mean into the next step.`}),(0,T.jsx)(`div`,{className:`diffusion-phase-buttons`,children:Rl.map(n=>(0,T.jsxs)(`button`,{type:`button`,"aria-pressed":e===n.value,onClick:()=>t(n.value),children:[(0,T.jsx)(`span`,{children:n.shortLabel}),(0,T.jsx)(`strong`,{children:n.label})]},n.value))}),(0,T.jsxs)(`div`,{className:`diffusion-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected state`}),(0,T.jsx)(`strong`,{children:Rl[r].label}),(0,T.jsxs)(`span`,{children:[`visible scalar = `,V(d)]}),(0,T.jsx)(`span`,{children:i?`updated denoiser`:`initial denoiser`})]})]})]})}var Bl={generator:{weight:.2,bias:0},discriminator:{weight:1,bias:0}},Vl=.5,Hl=.25;function H(e){return Math.abs(e)<1e-12?0:e}function U(e){if(e>=0)return 1/(1+Math.exp(-e));let t=Math.exp(e);return t/(1+t)}function W(e,t,n){let r=H(t*n.generator.weight),i=H(r+n.generator.bias),a=H(e*n.discriminator.weight+n.discriminator.bias),o=H(i*n.discriminator.weight+n.discriminator.bias),s=U(a),c=U(o);return{generatorProduct:r,fakeSample:i,realLogit:a,realProbability:s,fakeLogit:o,fakeProbability:c,discriminatorLoss:-.5*(Math.log(s)+Math.log(1-c)),generatorLoss:-Math.log(c)}}function G(e,t){let n=1e-6;return{epsilon:n,numerical:e.map((r,i)=>{let a=[...e],o=[...e];return a[i]+=n,o[i]-=n,(t(a)-t(o))/(2*n)}),maxAbsoluteError:0}}function Ul(e,t){return Math.max(...e.map((e,n)=>Math.abs(e-t[n])))}function Wl(e=1,t=1,n=Vl,r=Hl,i=Bl){if(![e,t,n,r,i.generator.weight,i.generator.bias,i.discriminator.weight,i.discriminator.bias].every(Number.isFinite)||n<=0||r<=0)throw Error(`NN18 V1 needs finite scalar samples and parameters, plus positive learning rates.`);let a={generator:{...i.generator},discriminator:{...i.discriminator}},o=W(e,t,a),s=.5*(o.realProbability-1),c=.5*o.fakeProbability,l=H(s*e+c*o.fakeSample),u=H(s+c),d=[l,u],f=G([a.discriminator.weight,a.discriminator.bias],([t,n])=>{let r=U(e*t+n),i=U(o.fakeSample*t+n);return-.5*(Math.log(r)+Math.log(1-i))}),p={weight:H(a.discriminator.weight-n*l),bias:H(a.discriminator.bias-n*u)},m=W(e,t,{generator:a.generator,discriminator:p}),h=m.fakeProbability-1,g=H(h*p.weight),_=H(g*t),v=g,y=[_,v],b=G([a.generator.weight,a.generator.bias],([e,n])=>{let r=U((t*e+n)*p.weight+p.bias);return-Math.log(r)}),x={weight:H(a.generator.weight-r*_),bias:H(a.generator.bias-r*v)},ee=W(e,t,{generator:x,discriminator:p});return{realSample:e,savedNoise:t,discriminatorLearningRate:n,generatorLearningRate:r,parameters:a,initial:o,discriminatorStep:{backward:{realLogitGradient:s,fakeLogitGradient:c,weightGradient:l,biasGradient:u,fakeSampleGradient:0},updatedParameters:p,state:m,gradientCheck:{epsilon:f.epsilon,parameterOrder:[`discriminator.weight`,`discriminator.bias`],analytical:d,numerical:f.numerical,maxAbsoluteError:Ul(d,f.numerical)}},generatorStep:{backward:{fakeLogitGradient:h,fakeSampleGradient:g,weightGradient:_,biasGradient:v},updatedParameters:x,state:ee,gradientCheck:{epsilon:b.epsilon,parameterOrder:[`generator.weight`,`generator.bias`],analytical:y,numerical:b.numerical,maxAbsoluteError:Ul(y,b.numerical)}}}}var Gl=[{value:`initial`,label:`Before training`,shortLabel:`0. Forward`},{value:`discriminator`,label:`Discriminator moves`,shortLabel:`1. Critic`},{value:`generator`,label:`Generator responds`,shortLabel:`2. Maker`}];function K(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function Kl(e){return e===`discriminator`?`The fake sample is detached. Only the discriminator can move.`:e===`generator`?`The updated discriminator is frozen. Its input gradient teaches the generator.`:`Both players make predictions, but neither has moved yet.`}function ql({state:e,realSample:t}){let n=e=>`${Math.max(3,Math.min(97,e*72+12))}%`;return(0,T.jsxs)(`div`,{className:`gan-number-line`,"aria-label":`GAN sample number line`,children:[(0,T.jsxs)(`div`,{className:`gan-number-line__axis`,"aria-hidden":`true`,children:[(0,T.jsx)(`span`,{children:`0`}),(0,T.jsx)(`span`,{children:`0.5`}),(0,T.jsx)(`span`,{children:`1`})]}),(0,T.jsxs)(`div`,{className:`gan-number-line__marker gan-number-line__marker--fake`,style:{left:n(e.fakeSample)},children:[(0,T.jsxs)(`strong`,{children:[`fake `,K(e.fakeSample)]}),(0,T.jsx)(`small`,{children:`G(noise)`})]}),(0,T.jsxs)(`div`,{className:`gan-number-line__marker gan-number-line__marker--real`,style:{left:n(t)},children:[(0,T.jsxs)(`strong`,{children:[`real `,K(t)]}),(0,T.jsx)(`small`,{children:`data`})]})]})}function Jl(){let[e,t]=(0,l.useState)(`initial`),n=(0,l.useMemo)(()=>Wl(),[]),r=e===`initial`?n.initial:e===`discriminator`?n.discriminatorStep.state:n.generatorStep.state,i=e===`initial`?n.parameters.discriminator:n.discriminatorStep.updatedParameters,a=e===`generator`?n.generatorStep.updatedParameters:n.parameters.generator;return(0,T.jsxs)(`main`,{className:`workspace workspace--gan`,children:[(0,T.jsxs)(`section`,{className:`gan-stage`,"aria-label":`One-dimensional GAN game trace`,children:[(0,T.jsxs)(`div`,{className:`gan-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN18 - two losses, two turns, one game`}),(0,T.jsx)(`h2`,{children:`A generator and discriminator on one number line`}),(0,T.jsx)(`p`,{children:`The critic learns to separate one real point from one generated point. Then the maker follows the frozen critic's slope toward a more convincing sample.`})]}),(0,T.jsx)(`div`,{className:`gan-chip`,children:`D moves -> freeze D -> G moves`})]}),(0,T.jsxs)(`section`,{className:`gan-sample-panel`,"aria-label":`GAN samples and discriminator probabilities`,children:[(0,T.jsxs)(`div`,{className:`gan-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same saved noise through every phase`}),(0,T.jsx)(`h2`,{children:`Watch the fake sample move toward the data`})]}),(0,T.jsx)(`code`,{children:Kl(e)})]}),(0,T.jsx)(ql,{state:r,realSample:n.realSample}),(0,T.jsxs)(`div`,{className:`gan-probability-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`critic on real`}),(0,T.jsxs)(`code`,{children:[`sigmoid(`,K(r.realLogit),`)`]}),(0,T.jsx)(`strong`,{children:K(r.realProbability)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`critic on fake`}),(0,T.jsxs)(`code`,{children:[`sigmoid(`,K(r.fakeLogit),`)`]}),(0,T.jsx)(`strong`,{children:K(r.fakeProbability)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`generator equation`}),(0,T.jsxs)(`code`,{children:[K(n.savedNoise),` x `,K(a.weight),` + `,K(a.bias)]}),(0,T.jsx)(`strong`,{children:K(r.fakeSample)})]})]})]}),(0,T.jsxs)(`section`,{className:`gan-objective-panel`,"aria-label":`GAN competing objectives`,children:[(0,T.jsxs)(`div`,{className:`gan-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`The players do not minimize one shared loss`}),(0,T.jsx)(`h2`,{children:`Judge correctly; fool the judge`})]}),(0,T.jsx)(`code`,{children:`non-saturating generator objective`})]}),(0,T.jsxs)(`div`,{className:`gan-objectives`,children:[(0,T.jsxs)(`div`,{className:e===`discriminator`?`gan-player gan-player--active`:`gan-player`,children:[(0,T.jsx)(`small`,{children:`discriminator minimizes`}),(0,T.jsx)(`code`,{children:`-0.5 x [log D(real) + log(1 - D(fake))]`}),(0,T.jsxs)(`strong`,{children:[`D loss `,K(r.discriminatorLoss)]}),(0,T.jsx)(`span`,{children:`real label 1, fake label 0`})]}),(0,T.jsx)(`div`,{className:`gan-versus`,"aria-hidden":`true`,children:`vs`}),(0,T.jsxs)(`div`,{className:e===`generator`?`gan-player gan-player--active gan-player--generator`:`gan-player gan-player--generator`,children:[(0,T.jsx)(`small`,{children:`generator minimizes`}),(0,T.jsx)(`code`,{children:`-log D(G(noise))`}),(0,T.jsxs)(`strong`,{children:[`G loss `,K(r.generatorLoss)]}),(0,T.jsx)(`span`,{children:`make the fake receive label 1`})]})]})]}),(0,T.jsxs)(`section`,{className:`gan-gradient-panel`,"aria-label":`GAN active gradient route`,children:[(0,T.jsxs)(`div`,{className:`gan-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Only one parameter set moves per turn`}),(0,T.jsx)(`h2`,{children:e===`generator`?`The critic becomes a teaching signal`:e===`discriminator`?`The generated value is detached`:`Choose a move to reveal its gradient`})]}),(0,T.jsx)(`code`,{children:e===`initial`?`forward pass only`:`active route highlighted`})]}),e===`initial`?(0,T.jsx)(`div`,{className:`gan-gradient-placeholder`,children:`Start with two sigmoid scores. The turn buttons expose which edges carry gradients and which parameter set stays frozen.`}):e===`discriminator`?(0,T.jsxs)(`div`,{className:`gan-gradient-route`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`real-logit route`}),(0,T.jsx)(`code`,{children:`0.5 x (D(real) - 1)`}),(0,T.jsx)(`strong`,{children:K(n.discriminatorStep.backward.realLogitGradient)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`fake-logit route`}),(0,T.jsx)(`code`,{children:`0.5 x D(fake)`}),(0,T.jsx)(`strong`,{children:K(n.discriminatorStep.backward.fakeLogitGradient)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`gan-gradient-route__result`,children:[(0,T.jsx)(`small`,{children:`D weight / bias gradient`}),(0,T.jsxs)(`strong`,{children:[K(n.discriminatorStep.backward.weightGradient),` / `,K(n.discriminatorStep.backward.biasGradient)]}),(0,T.jsx)(`span`,{children:`gradient into fake = 0 (detached)`})]})]}):(0,T.jsxs)(`div`,{className:`gan-gradient-route`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`G loss to fake logit`}),(0,T.jsx)(`code`,{children:`D(fake) - 1`}),(0,T.jsx)(`strong`,{children:K(n.generatorStep.backward.fakeLogitGradient)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`x`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`frozen D input slope`}),(0,T.jsxs)(`code`,{children:[`D weight = `,K(i.weight)]}),(0,T.jsx)(`strong`,{children:K(n.generatorStep.backward.fakeSampleGradient)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`gan-gradient-route__result gan-gradient-route__result--generator`,children:[(0,T.jsx)(`small`,{children:`G weight / bias gradient`}),(0,T.jsxs)(`strong`,{children:[K(n.generatorStep.backward.weightGradient),` / `,K(n.generatorStep.backward.biasGradient)]}),(0,T.jsx)(`span`,{children:`D parameters stay frozen`})]})]})]}),(0,T.jsxs)(`section`,{className:`gan-update-panel`,"aria-label":`GAN alternating updates and gradient audits`,children:[(0,T.jsxs)(`div`,{className:`gan-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Audit each player against its own objective`}),(0,T.jsx)(`h2`,{children:`The losses push back after alternating moves`})]}),(0,T.jsx)(`code`,{children:`central difference epsilon = 1e-6`})]}),(0,T.jsxs)(`div`,{className:`gan-update-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`discriminator update`}),(0,T.jsxs)(`code`,{children:[`w `,K(n.parameters.discriminator.weight),` -> `,K(n.discriminatorStep.updatedParameters.weight)]}),(0,T.jsxs)(`code`,{children:[`b `,K(n.parameters.discriminator.bias),` -> `,K(n.discriminatorStep.updatedParameters.bias)]}),(0,T.jsxs)(`strong`,{children:[`D loss `,K(n.initial.discriminatorLoss),` -> `,K(n.discriminatorStep.state.discriminatorLoss)]}),(0,T.jsxs)(`span`,{children:[`max audit error `,n.discriminatorStep.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`generator counter-move`}),(0,T.jsxs)(`code`,{children:[`w `,K(n.parameters.generator.weight),` -> `,K(n.generatorStep.updatedParameters.weight)]}),(0,T.jsxs)(`code`,{children:[`b `,K(n.parameters.generator.bias),` -> `,K(n.generatorStep.updatedParameters.bias)]}),(0,T.jsxs)(`strong`,{children:[`G loss `,K(n.discriminatorStep.state.generatorLoss),` -> `,K(n.generatorStep.state.generatorLoss)]}),(0,T.jsxs)(`span`,{children:[`max audit error `,n.generatorStep.gradientCheck.maxAbsoluteError.toExponential(3)]})]})]}),(0,T.jsxs)(`div`,{className:`gan-counterpush`,children:[(0,T.jsxs)(`strong`,{children:[`After G moves, D loss rises to `,K(n.generatorStep.state.discriminatorLoss),`.`]}),(0,T.jsx)(`p`,{children:`That is the game working: the newly improved fake is harder for the frozen critic.`})]})]})]}),(0,T.jsxs)(`aside`,{className:`gan-controls`,"aria-label":`GAN game phase controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Alternating schedule`}),(0,T.jsx)(`h2`,{children:`Advance one turn`}),(0,T.jsx)(`p`,{children:`These are snapshots of one deterministic round, not three independent experiments.`}),(0,T.jsx)(`div`,{className:`gan-phase-buttons`,children:Gl.map(n=>(0,T.jsxs)(`button`,{type:`button`,"aria-pressed":e===n.value,onClick:()=>t(n.value),children:[(0,T.jsx)(`span`,{children:n.shortLabel}),(0,T.jsx)(`strong`,{children:n.label})]},n.value))}),(0,T.jsxs)(`div`,{className:`gan-selected-summary`,children:[(0,T.jsx)(`small`,{children:`current snapshot`}),(0,T.jsx)(`strong`,{children:Gl.find(t=>t.value===e).label}),(0,T.jsxs)(`span`,{children:[`fake = `,K(r.fakeSample)]}),(0,T.jsxs)(`span`,{children:[`D(fake) = `,K(r.fakeProbability)]})]}),(0,T.jsxs)(`div`,{className:`gan-freeze-key`,children:[(0,T.jsx)(`small`,{children:`freeze contract`}),(0,T.jsx)(`code`,{children:e===`discriminator`?`grad(G) = 0`:e===`generator`?`grad(D params) = 0`:`no backward pass`})]})]})]})}var Yl={encoder:{mean:{weight:.4,bias:0},logVariance:{weight:0,bias:0}},decoder:{weight:1,bias:0}},Xl=.5,Zl=.1,Ql=.1,$l=[`encoder.mean.weight`,`encoder.mean.bias`,`encoder.log_variance.weight`,`encoder.log_variance.bias`,`decoder.weight`,`decoder.bias`];function eu(e){return Math.abs(e)<1e-12?0:e}function tu(e){return{encoder:{mean:{...e.encoder.mean},logVariance:{...e.encoder.logVariance}},decoder:{...e.decoder}}}function nu(e,t,n,r){let i=eu(e*t.encoder.mean.weight),a=eu(i+t.encoder.mean.bias),o=eu(e*t.encoder.logVariance.weight),s=eu(o+t.encoder.logVariance.bias),c=Math.exp(s),l=Math.exp(.5*s),u=eu(l*n),d=eu(a+u),f=eu(d*t.decoder.weight),p=eu(f+t.decoder.bias),m=eu(p-e),h=.5*m*m,g=a*a,_=.5*(g+c-1-s),v=r*_;return{meanProduct:i,mean:a,logVarianceProduct:o,logVariance:s,variance:c,standardDeviation:l,epsilon:n,noiseContribution:u,latent:d,decoderProduct:f,reconstruction:p,error:m,reconstructionLoss:h,meanSquared:g,kl:_,weightedKl:v,totalLoss:h+v}}function ru(e){return[e.encoder.mean.weight,e.encoder.mean.bias,e.encoder.logVariance.weight,e.encoder.logVariance.bias,e.decoder.weight,e.decoder.bias]}function iu(e){return{encoder:{mean:{weight:e[0],bias:e[1]},logVariance:{weight:e[2],bias:e[3]}},decoder:{weight:e[4],bias:e[5]}}}function au(e=Zl,t=Xl,n=Ql,r=1,i=Yl){let a=ru(i);if(!Number.isFinite(e)||e<0||!Number.isFinite(t)||!Number.isFinite(n)||n<=0||!Number.isFinite(r)||!a.every(Number.isFinite))throw Error(`NN17 V1 needs finite scalar parameters, input and epsilon, non-negative beta, and a positive learning rate.`);let o=tu(i),s=nu(r,o,t,e);if(!Number.isFinite(s.variance)||!Number.isFinite(s.standardDeviation)||!Number.isFinite(s.totalLoss))throw Error(`NN17 V1 produced a non-finite Gaussian or objective.`);let c=s.error,l=eu(c*s.latent),u=c,d=eu(c*o.decoder.weight),f=d,p=eu(d*.5*s.standardDeviation*t),m=s.mean,h=eu(.5*(s.variance-1)),g=eu(e*m),_=eu(e*h),v=eu(f+g),y=eu(p+_),b=eu(v*r),x=v,ee=eu(y*r),S=y,C={reconstructionGradient:c,decoderWeightGradient:l,decoderBiasGradient:u,latentGradient:d,reconstructionMeanGradient:f,reconstructionLogVarianceGradient:p,klMeanGradient:m,klLogVarianceGradient:h,weightedKlMeanGradient:g,weightedKlLogVarianceGradient:_,meanGradient:v,logVarianceGradient:y,meanWeightGradient:b,meanBiasGradient:x,logVarianceWeightGradient:ee,logVarianceBiasGradient:S},te=[b,x,ee,S,l,u],ne=1e-6,w=a.map((n,i)=>{let o=[...a],s=[...a];return o[i]+=ne,s[i]-=ne,(nu(r,iu(o),t,e).totalLoss-nu(r,iu(s),t,e).totalLoss)/(2*ne)}),re=Math.max(...te.map((e,t)=>Math.abs(e-w[t]))),ie=iu(a.map((e,t)=>e-n*te[t])),ae=nu(r,ie,t,e);return{input:r,beta:e,samplingEpsilon:t,learningRate:n,parameters:o,forward:s,backward:C,gradientCheck:{epsilon:ne,parameterOrder:[...$l],analytical:te,numerical:w,maxAbsoluteError:re},updatedParameters:ie,postUpdate:ae}}var ou=[0,.1,.25,1];function q(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function su(){let[e,t]=(0,l.useState)(.1),[n,r]=(0,l.useState)(`mean`),[i,a]=(0,l.useState)(!1),o=(0,l.useMemo)(()=>au(e),[e]),s=i?o.postUpdate:o.forward,c=i?o.updatedParameters:o.parameters,u=n===`mean`?o.backward.reconstructionMeanGradient:o.backward.reconstructionLogVarianceGradient,d=n===`mean`?o.backward.weightedKlMeanGradient:o.backward.weightedKlLogVarianceGradient,f=n===`mean`?o.backward.meanGradient:o.backward.logVarianceGradient,p=n===`mean`?`mean`:`log-variance`;return(0,T.jsxs)(`main`,{className:`workspace workspace--variational`,children:[(0,T.jsxs)(`section`,{className:`variational-stage`,"aria-label":`Scalar variational autoencoder trace`,children:[(0,T.jsxs)(`div`,{className:`variational-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN17 - uncertainty without hidden randomness`}),(0,T.jsx)(`h2`,{children:`One Gaussian latent sample, fully unpacked`}),(0,T.jsx)(`p`,{children:`Encode a mean and log-variance, transform one saved noise value, then watch reconstruction and prior matching negotiate one update.`})]}),(0,T.jsx)(`div`,{className:`variational-chip`,children:`mean + sigma x epsilon`})]}),(0,T.jsxs)(`section`,{className:`variational-flow-panel`,"aria-label":`Variational encode sample and decode path`,children:[(0,T.jsxs)(`div`,{className:`variational-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`The sample is random; the path is differentiable`}),(0,T.jsx)(`h2`,{children:`Move noise outside the network`})]}),(0,T.jsx)(`code`,{children:i?`after one SGD step`:`saved epsilon = 0.5`})]}),(0,T.jsxs)(`div`,{className:`variational-flow`,children:[(0,T.jsxs)(`div`,{className:`variational-scalar-node`,children:[(0,T.jsx)(`small`,{children:`input is target`}),(0,T.jsxs)(`strong`,{children:[`x = `,q(o.input)]})]}),(0,T.jsx)(`span`,{className:`variational-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`variational-distribution-node`,children:[(0,T.jsx)(`small`,{children:`encoder distribution`}),(0,T.jsxs)(`code`,{children:[`mean = `,q(s.meanProduct),` + `,q(c.encoder.mean.bias),` = `,q(s.mean)]}),(0,T.jsxs)(`code`,{children:[`log var = `,q(s.logVarianceProduct),` + `,q(c.encoder.logVariance.bias),` = `,q(s.logVariance)]}),(0,T.jsxs)(`code`,{children:[`sigma = `,q(s.standardDeviation)]})]}),(0,T.jsx)(`span`,{className:`variational-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`variational-sample-node`,children:[(0,T.jsx)(`small`,{children:`reparameterized sample`}),(0,T.jsxs)(`code`,{children:[q(s.mean),` + `,q(s.standardDeviation),` x `,q(s.epsilon)]}),(0,T.jsxs)(`strong`,{children:[`z = `,q(s.latent)]}),(0,T.jsx)(`span`,{children:`epsilon stays fixed for this audit`})]}),(0,T.jsx)(`span`,{className:`variational-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{className:`variational-scalar-node variational-scalar-node--output`,children:[(0,T.jsx)(`small`,{children:`decoder reconstruction`}),(0,T.jsxs)(`code`,{children:[q(s.latent),` x `,q(c.decoder.weight),` + `,q(c.decoder.bias)]}),(0,T.jsxs)(`strong`,{children:[`x_hat = `,q(s.reconstruction)]})]})]})]}),(0,T.jsxs)(`section`,{className:`variational-objective-panel`,"aria-label":`Variational reconstruction and KL objective`,children:[(0,T.jsxs)(`div`,{className:`variational-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Two pressures, one weighted objective`}),(0,T.jsx)(`h2`,{children:`Reconstruct here; stay sampleable everywhere`})]}),(0,T.jsxs)(`div`,{className:`variational-loss-badge`,children:[(0,T.jsx)(`small`,{children:`total loss`}),(0,T.jsx)(`strong`,{children:q(s.totalLoss)})]})]}),(0,T.jsxs)(`div`,{className:`variational-objective-equation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`reconstruction`}),(0,T.jsxs)(`code`,{children:[`0.5 x (`,q(s.error),`)^2`]}),(0,T.jsx)(`strong`,{children:q(s.reconstructionLoss)}),(0,T.jsx)(`span`,{children:`preserve this input`})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`KL to Normal(0, 1)`}),(0,T.jsxs)(`code`,{children:[`0.5 x (`,q(s.meanSquared),` + `,q(s.variance),` - 1 - `,q(s.logVariance),`)`]}),(0,T.jsx)(`strong`,{children:q(s.kl)}),(0,T.jsx)(`span`,{children:`keep latent space sampleable`})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`x`}),(0,T.jsxs)(`div`,{className:`variational-beta-node`,children:[(0,T.jsx)(`small`,{children:`beta`}),(0,T.jsx)(`strong`,{children:q(e)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,T.jsxs)(`div`,{className:`variational-total-node`,children:[(0,T.jsx)(`small`,{children:`weighted total`}),(0,T.jsxs)(`code`,{children:[q(s.reconstructionLoss),` + `,q(s.weightedKl)]}),(0,T.jsx)(`strong`,{children:q(s.totalLoss)})]})]})]}),(0,T.jsxs)(`section`,{className:`variational-gradient-panel`,"aria-label":`Variational ${p} gradient tradeoff`,children:[(0,T.jsxs)(`div`,{className:`variational-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Both objectives meet at the encoder`}),(0,T.jsx)(`h2`,{children:`Beta can reinforce, soften, or reverse a direction`})]}),(0,T.jsx)(`code`,{children:`saved forward pass gradients`})]}),(0,T.jsxs)(`div`,{className:`variational-gradient-targets`,"aria-label":`Variational gradient target`,children:[(0,T.jsx)(`button`,{"aria-pressed":n===`mean`,type:`button`,onClick:()=>r(`mean`),children:`mean output`}),(0,T.jsx)(`button`,{"aria-pressed":n===`logVariance`,type:`button`,onClick:()=>r(`logVariance`),children:`log-variance output`})]}),(0,T.jsxs)(`div`,{className:`variational-gradient-routes`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`reconstruction route`}),(0,T.jsx)(`strong`,{children:q(u)}),(0,T.jsx)(`span`,{children:`sample should rebuild x`})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`beta x KL route`}),(0,T.jsxs)(`code`,{children:[q(e),` x `,q(n===`mean`?o.backward.klMeanGradient:o.backward.klLogVarianceGradient)]}),(0,T.jsx)(`strong`,{children:q(d)}),(0,T.jsx)(`span`,{children:`distribution should match prior`})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,T.jsxs)(`div`,{className:`variational-combined-gradient`,children:[(0,T.jsxs)(`small`,{children:[`combined `,p,` gradient`]}),(0,T.jsx)(`strong`,{children:q(f)}),(0,T.jsx)(`span`,{children:f===0?`the routes cancel exactly`:`this is the encoder's update direction`})]})]}),(0,T.jsxs)(`div`,{className:`variational-gradient-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder weight`}),(0,T.jsx)(`code`,{children:q(o.backward.decoderWeightGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder bias`}),(0,T.jsx)(`code`,{children:q(o.backward.decoderBiasGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean weight / bias`}),(0,T.jsxs)(`code`,{children:[q(o.backward.meanWeightGradient),` / `,q(o.backward.meanBiasGradient)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`log-var weight / bias`}),(0,T.jsxs)(`code`,{children:[q(o.backward.logVarianceWeightGradient),` / `,q(o.backward.logVarianceBiasGradient)]})]})]})]}),(0,T.jsxs)(`section`,{className:`variational-update-panel`,"aria-label":`Variational SGD update and gradient audit`,children:[(0,T.jsxs)(`div`,{className:`variational-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Same epsilon for analytical and numerical slopes`}),(0,T.jsx)(`h2`,{children:`Audit six parameters, then rerun everything`})]}),(0,T.jsxs)(`code`,{children:[`parameter - `,o.learningRate,` x gradient`]})]}),(0,T.jsxs)(`div`,{className:`variational-parameter-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`mean head before -> after`}),(0,T.jsxs)(`code`,{children:[`w `,q(o.parameters.encoder.mean.weight),` -> `,q(o.updatedParameters.encoder.mean.weight)]}),(0,T.jsxs)(`code`,{children:[`b `,q(o.parameters.encoder.mean.bias),` -> `,q(o.updatedParameters.encoder.mean.bias)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`log-var head before -> after`}),(0,T.jsxs)(`code`,{children:[`w `,q(o.parameters.encoder.logVariance.weight),` -> `,q(o.updatedParameters.encoder.logVariance.weight)]}),(0,T.jsxs)(`code`,{children:[`b `,q(o.parameters.encoder.logVariance.bias),` -> `,q(o.updatedParameters.encoder.logVariance.bias)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`decoder before -> after`}),(0,T.jsxs)(`code`,{children:[`w `,q(o.parameters.decoder.weight),` -> `,q(o.updatedParameters.decoder.weight)]}),(0,T.jsxs)(`code`,{children:[`b `,q(o.parameters.decoder.bias),` -> `,q(o.updatedParameters.decoder.bias)]})]})]}),(0,T.jsxs)(`div`,{className:`variational-audit-row`,children:[(0,T.jsx)(`span`,{children:`Central finite differences - 6 parameters`}),(0,T.jsxs)(`code`,{children:[`epsilon = `,o.gradientCheck.epsilon]}),(0,T.jsxs)(`strong`,{children:[`max error `,o.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,T.jsxs)(`div`,{className:`variational-loss-drop`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`total before`}),(0,T.jsx)(`strong`,{children:q(o.forward.totalLoss)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`total after`}),(0,T.jsx)(`strong`,{children:q(o.postUpdate.totalLoss)})]}),(0,T.jsxs)(`p`,{children:[`Reconstruction falls from `,q(o.forward.reconstructionLoss),` to `,q(o.postUpdate.reconstructionLoss),`; KL may move differently while the selected weighted objective falls.`]})]})]})]}),(0,T.jsxs)(`aside`,{className:`variational-controls`,"aria-label":`Variational trace controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Turn the prior pressure`}),(0,T.jsx)(`h2`,{children:`KL tradeoff controls`}),(0,T.jsx)(`p`,{children:`Epsilon stays fixed at 0.5. Changing beta therefore changes the objective and gradient, not the sampled noise.`}),(0,T.jsx)(`div`,{className:`variational-beta-buttons`,"aria-label":`Variational beta selection`,children:ou.map(n=>(0,T.jsxs)(`button`,{"aria-pressed":e===n,type:`button`,onClick:()=>{t(n),a(!1)},children:[`beta `,n]},n))}),(0,T.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:i,onChange:e=>a(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Use updated parameters`}),(0,T.jsx)(`small`,{children:`Rerun distribution, sample, decoder, and both losses.`})]})]}),(0,T.jsxs)(`div`,{className:`variational-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected beta`}),(0,T.jsx)(`strong`,{children:q(e)}),(0,T.jsxs)(`span`,{children:[`mean gradient `,q(o.backward.meanGradient),`; total `,q(o.forward.totalLoss)]})]}),(0,T.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,T.jsx)(`span`,{children:`Why save epsilon?`}),(0,T.jsx)(`p`,{children:`The trace remains stochastic in meaning but reproducible in execution. Finite differences compare the same noise on both sides.`})]}),(0,T.jsxs)(`div`,{className:`attention-next-note`,children:[(0,T.jsx)(`span`,{children:`Do not optimize one term alone`}),(0,T.jsx)(`p`,{children:`A useful VAE needs reconstruction and a navigable latent prior. Their weighted sum, not either isolated term, defines this step.`})]})]})]})}function cu(){let[e,t]=(0,l.useState)(`autoencoder`);return(0,T.jsxs)(`div`,{className:`representation-workbench`,children:[(0,T.jsxs)(`nav`,{className:`representation-lab-switch`,"aria-label":`Representation learning lab`,children:[(0,T.jsx)(`button`,{"aria-pressed":e===`autoencoder`,type:`button`,onClick:()=>t(`autoencoder`),children:`Deterministic bottleneck`}),(0,T.jsx)(`button`,{"aria-pressed":e===`variational`,type:`button`,onClick:()=>t(`variational`),children:`Variational sample`}),(0,T.jsx)(`button`,{"aria-pressed":e===`gan`,type:`button`,onClick:()=>t(`gan`),children:`Adversarial game`}),(0,T.jsx)(`button`,{"aria-pressed":e===`diffusion`,type:`button`,onClick:()=>t(`diffusion`),children:`Diffusion path`})]}),e===`autoencoder`?(0,T.jsx)(Dl,{}):e===`variational`?(0,T.jsx)(su,{}):e===`gan`?(0,T.jsx)(Jl,{}):(0,T.jsx)(zl,{})]})}var lu=[1,0,2,0,1],uu=[[1,1,1],[1,1,1]];function du(e){return e===0?0:e}function fu(e,t){if(e.length===0||t.length===0||t.length%2==0||![...e,...t].every(Number.isFinite))throw Error(`Same correlation needs a finite signal and an odd kernel.`);let n=Math.floor(t.length/2);return e.map((r,i)=>du(t.reduce((t,r,a)=>{let o=i+a-n;return t+(o>=0&&o<e.length?e[o]:0)*r},0)))}function pu(e=lu,t=uu){if(t.length!==2||t.some(e=>e.length!==3||e.some(e=>e!==1)))throw Error(`NN08 V1 uses two [1, 1, 1] kernels.`);let n=fu(e,t[0]),r=fu(n,t[1]),i=[...e],a=r.map((e,t)=>du(e+i[t])),o=a.map(e=>Math.max(0,e));return{hidden:n,main:r,skip:i,residualSum:a,output:o,traces:e.map((t,s)=>{let c=[s-1,s,s+1].filter(t=>t>=0&&t<e.length),l=e.map(()=>0),u=c.map(t=>{let r=[t-1,t,t+1].filter(t=>t>=0&&t<e.length);return r.forEach(e=>{l[e]=l[e]+1}),{hiddenIndex:t,inputIndices:r,inputValues:r.map(t=>e[t]),subtotal:n[t]}});return{outputIndex:s,hiddenIndices:c,hiddenValues:c.map(e=>n[e]),hiddenPaths:u,inputPathCounts:l,inputContributions:e.map((e,t)=>du(e*l[t])),receptiveFieldIndices:l.map((e,t)=>({count:e,inputIndex:t})).filter(({count:e})=>e>0).map(({inputIndex:e})=>e),mainOutput:r[s],skipContribution:i[s],residualSum:a[s],output:o[s]}})}}function mu(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function hu({label:e,values:t,selectedIndex:n,activeIndices:r=[],annotation:i}){return(0,T.jsxs)(`div`,{className:`residual-signal-block`,children:[(0,T.jsxs)(`div`,{className:`residual-row-label`,children:[(0,T.jsx)(`span`,{children:e}),i===void 0?null:(0,T.jsx)(`code`,{children:i})]}),(0,T.jsx)(`div`,{className:`residual-signal-row`,style:{gridTemplateColumns:`repeat(${t.length}, minmax(52px, 1fr))`},"aria-label":e,children:t.map((e,t)=>(0,T.jsxs)(`div`,{className:t===n?`residual-cell residual-cell--selected`:r.includes(t)?`residual-cell residual-cell--active`:`residual-cell`,children:[(0,T.jsxs)(`small`,{children:[`[`,t,`]`]}),(0,T.jsx)(`strong`,{children:mu(e)})]},t))})]})}function gu(){let[e,t]=(0,l.useState)(2),[n,r]=(0,l.useState)(!0),i=(0,l.useMemo)(()=>pu(),[]),a=i.traces[e],o=a.mainOutput+(n?a.skipContribution:0),s=Math.max(0,o),c=i.main.map((e,t)=>Math.max(0,e+(n?i.skip[t]:0)));function u(){t(2),r(!0)}return(0,T.jsxs)(`main`,{className:`workspace workspace--residual`,children:[(0,T.jsxs)(`section`,{className:`residual-stage`,"aria-label":`Residual path and receptive field trace`,children:[(0,T.jsxs)(`div`,{className:`residual-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN08 · spatial networks`}),(0,T.jsx)(`h2`,{children:`Residual-path microscope`}),(0,T.jsx)(`p`,{children:`Open one output into its deep local path and short identity path, then trace every dependency back to the original input.`})]}),(0,T.jsx)(`div`,{className:`residual-shape-chip`,children:`5 → 5 → 5 + identity`})]}),(0,T.jsxs)(`section`,{className:`residual-block-panel`,"aria-label":`Residual block forward trace`,children:[(0,T.jsxs)(`div`,{className:`residual-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Selected output · y[`,e,`]`]}),(0,T.jsx)(`h2`,{children:`Two routes meet at one addition`})]}),(0,T.jsx)(`strong`,{className:`residual-result`,children:mu(s)})]}),(0,T.jsxs)(`div`,{className:`residual-main-path`,children:[(0,T.jsx)(`span`,{className:`residual-lane-label`,children:`main path · two local layers`}),(0,T.jsx)(hu,{label:`input x`,values:lu,selectedIndex:n?e:void 0,activeIndices:a.receptiveFieldIndices,annotation:`receptive field highlighted`}),(0,T.jsx)(`span`,{className:`residual-down-arrow`,"aria-hidden":`true`,children:`↓ [1, 1, 1] · same zero pad`}),(0,T.jsx)(hu,{label:`hidden h`,values:i.hidden,activeIndices:a.hiddenIndices,annotation:`${a.hiddenIndices.length} values feed main[${e}]`}),(0,T.jsx)(`span`,{className:`residual-down-arrow`,"aria-hidden":`true`,children:`↓ [1, 1, 1] · same zero pad`}),(0,T.jsx)(hu,{label:`main transform F(x)`,values:i.main,selectedIndex:e,annotation:`main[${e}] = ${mu(a.mainOutput)}`})]}),(0,T.jsxs)(`div`,{className:n?`residual-skip-lane`:`residual-skip-lane residual-skip-lane--disabled`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`identity skip`}),(0,T.jsxs)(`strong`,{children:[`x[`,e,`] = `,mu(a.skipContribution)]})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`────────────→`}),(0,T.jsx)(`code`,{children:n?`included`:`disabled`})]}),(0,T.jsxs)(`div`,{className:`residual-addition`,"aria-label":`Selected residual addition`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`main path`}),(0,T.jsx)(`strong`,{children:mu(a.mainOutput)})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`skip path`}),(0,T.jsx)(`strong`,{children:n?mu(a.skipContribution):`0`})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`before ReLU`}),(0,T.jsx)(`strong`,{children:mu(o)})]}),(0,T.jsx)(`span`,{children:`→`}),(0,T.jsxs)(`div`,{className:`residual-addition__output`,children:[(0,T.jsx)(`small`,{children:`output`}),(0,T.jsx)(`strong`,{children:mu(s)})]})]}),(0,T.jsx)(hu,{label:n?`block output ReLU(F(x) + x)`:`block output ReLU(F(x))`,values:c,selectedIndex:e})]}),(0,T.jsxs)(`section`,{className:`receptive-panel`,"aria-label":`Receptive field explorer`,children:[(0,T.jsxs)(`div`,{className:`residual-panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Receptive field · output `,e]}),(0,T.jsx)(`h2`,{children:`One output, every path back`})]}),(0,T.jsxs)(`div`,{className:`field-width-badge`,children:[(0,T.jsx)(`small`,{children:`in-range width`}),(0,T.jsx)(`strong`,{children:a.receptiveFieldIndices.length})]})]}),(0,T.jsx)(`div`,{className:`hidden-path-grid`,children:a.hiddenPaths.map(e=>(0,T.jsxs)(`article`,{className:`hidden-path-card`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`layer 2 reads`}),(0,T.jsxs)(`strong`,{children:[`h[`,e.hiddenIndex,`] = `,mu(e.subtotal)]})]}),(0,T.jsx)(`code`,{children:e.inputIndices.map(e=>`x[${e}]`).join(` + `)}),(0,T.jsxs)(`span`,{children:[e.inputValues.map(mu).join(` + `),` = `,mu(e.subtotal)]})]},e.hiddenIndex))}),(0,T.jsx)(`div`,{className:`path-count-table-wrap`,children:(0,T.jsxs)(`table`,{className:`path-count-table`,children:[(0,T.jsx)(`caption`,{children:`Original inputs after expanding both layers`}),(0,T.jsx)(`thead`,{children:(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`col`,children:`input`}),lu.map((e,t)=>(0,T.jsxs)(`th`,{scope:`col`,children:[`x[`,t,`]`]},t)),(0,T.jsx)(`th`,{scope:`col`,children:`sum`})]})}),(0,T.jsxs)(`tbody`,{children:[(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`paths`}),a.inputPathCounts.map((e,t)=>(0,T.jsx)(`td`,{children:e},t)),(0,T.jsx)(`td`,{children:`—`})]}),(0,T.jsxs)(`tr`,{children:[(0,T.jsx)(`th`,{scope:`row`,children:`value × paths`}),a.inputContributions.map((e,t)=>(0,T.jsx)(`td`,{children:mu(e)},t)),(0,T.jsx)(`td`,{className:`path-count-total`,children:mu(a.mainOutput)})]})]})]})}),(0,T.jsxs)(`div`,{className:`receptive-summary`,children:[(0,T.jsxs)(`code`,{children:[`receptive input indices = [`,a.receptiveFieldIndices.join(`, `),`]`]}),(0,T.jsx)(`span`,{children:`Zero-valued inputs still belong to the structural field: changing them can change this output.`})]})]})]}),(0,T.jsxs)(`aside`,{className:`residual-controls`,"aria-label":`Residual explorer controls`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Choose one output`}),(0,T.jsx)(`h2`,{children:`Trace controls`}),(0,T.jsx)(`p`,{children:`Move from a clipped boundary field to the five-position center field.`}),(0,T.jsx)(`div`,{className:`residual-output-buttons`,children:i.output.map((n,r)=>(0,T.jsxs)(`button`,{"aria-label":`Select residual output ${r}`,className:r===e?`residual-output-button residual-output-button--active`:`residual-output-button`,type:`button`,onClick:()=>t(r),children:[(0,T.jsxs)(`small`,{children:[`y[`,r,`]`]}),(0,T.jsx)(`strong`,{children:mu(n)})]},r))}),(0,T.jsxs)(`label`,{className:`residual-skip-control`,children:[(0,T.jsx)(`input`,{type:`checkbox`,checked:n,onChange:e=>r(e.target.checked)}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`strong`,{children:`Include identity skip`}),(0,T.jsx)(`small`,{children:`Add x[i] directly to the main path.`})]})]}),(0,T.jsxs)(`div`,{className:`button-grid`,children:[(0,T.jsx)(`button`,{type:`button`,disabled:e===0,onClick:()=>t(e=>Math.max(0,e-1)),children:`Previous output`}),(0,T.jsx)(`button`,{type:`button`,disabled:e===i.output.length-1,onClick:()=>t(e=>Math.min(i.output.length-1,e+1)),children:`Next output`}),(0,T.jsx)(`button`,{type:`button`,onClick:u,children:`Reset trace`})]}),(0,T.jsxs)(`div`,{className:`residual-note`,children:[(0,T.jsx)(`span`,{children:`What scales next?`}),(0,T.jsx)(`p`,{children:`More layers widen the main path's field. Projection skips handle shape changes, but must still land on a tensor compatible with the addition.`})]})]})]})}var _u=[1,-1,1,-1],vu=[-1,-1,1,-1],yu=[0,1,2,3];function bu(e){return Math.abs(e)<1e-12?0:e}function xu(e,t){if(e.length<2||e.some(e=>e!==-1&&e!==1))throw Error(`${t} must contain at least two bipolar values (-1 or +1).`)}function Su(e){let t=e.length;return e.map((n,r)=>e.map((e,i)=>r===i?0:n*e/t))}function Cu(e,t){let n=0;for(let r=0;r<e.length;r+=1)for(let i=0;i<e.length;i+=1)n+=t[r][i]*e[r]*e[i];return bu(-.5*n)}function wu(e,t){return e.reduce((e,n,r)=>e+n*t[r],0)/e.length}function Tu(e,t){return e.filter((e,n)=>e!==t[n]).length}function Eu(e=_u,t=vu,n=yu){if(xu(e,`storedPattern`),xu(t,`corruptedState`),e.length!==t.length||n.length!==e.length||new Set(n).size!==n.length||n.some(t=>!Number.isInteger(t)||t<0||t>=e.length))throw Error(`NN20 V1 needs equal-sized states and one permutation of every neuron index.`);let r=[...e],i=[...t],a=Su(r),o=Cu(i,a),s=wu(r,i),c=[],l=[...i];n.forEach((e,t)=>{let n=[...l],i=n.map((t,n)=>{let r=a[e][n];return{sourceIndex:n,weight:r,sourceState:t,contribution:bu(r*t)}}),o=bu(i.reduce((e,t)=>e+t.contribution,0)),s=n[e],u=o>0?1:o<0?-1:s;l=[...n],l[e]=u,c.push({step:t,neuronIndex:e,stateBefore:n,incoming:i,localField:o,previousState:s,nextState:u,changed:u!==s,stateAfter:[...l],energyBefore:Cu(n,a),energyAfter:Cu(l,a),overlapBefore:wu(r,n),overlapAfter:wu(r,l)})});let u=Cu(l,a),d=wu(r,l),f=Tu(r,l);return{storedPattern:r,normalization:r.length,weights:a,corruptedState:i,updateOrder:[...n],initialEnergy:o,initialOverlap:s,initialHammingDistance:Tu(r,i),updates:c,finalState:[...l],finalEnergy:u,finalOverlap:d,finalHammingDistance:f,converged:f===0&&c.every(e=>e.energyAfter<=e.energyBefore+1e-12)}}function Du(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):e.toFixed(2).replace(/0+$/,``).replace(/\.$/,``)}function Ou(e){return`[${e.map(e=>e>0?`+1`:`-1`).join(`, `)}]`}var ku=[{eyebrow:`0. Store`,title:`Hebbian weights`},{eyebrow:`1. Cue`,title:`One flipped bit`},{eyebrow:`2. Recall`,title:`Update neuron 0`},{eyebrow:`3. Recall`,title:`Update neuron 1`},{eyebrow:`4. Recall`,title:`Update neuron 2`},{eyebrow:`5. Recall`,title:`Update neuron 3`}];function Au(){let e=(0,l.useMemo)(()=>Eu(),[]),[t,n]=(0,l.useState)(0),r=Math.max(t-1,0),i=r>0?e.updates[r-1]:null,a=t===0?e.storedPattern:i?.stateAfter??e.corruptedState,o=t===0?e.finalEnergy:i?.energyAfter??e.initialEnergy,s=t===0?1:i?.overlapAfter??e.initialOverlap;return(0,T.jsxs)(`main`,{className:`workspace workspace--hopfield`,children:[(0,T.jsxs)(`section`,{className:`hopfield-stage`,"aria-label":`Hopfield associative memory trace`,children:[(0,T.jsxs)(`div`,{className:`hopfield-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN20 - a remembered pattern becomes an attractor`}),(0,T.jsx)(`h2`,{children:`Restore one flipped bit with four connected neurons`}),(0,T.jsx)(`p`,{children:`Store a bipolar pattern in symmetric weights, present a damaged cue, and audit every asynchronous update as energy moves downhill.`})]}),(0,T.jsx)(`div`,{className:`hopfield-chip`,children:`4 neurons - 1 memory`})]}),(0,T.jsxs)(`section`,{className:`hopfield-store-panel`,"aria-label":`Hopfield Hebbian storage rule`,children:[(0,T.jsxs)(`div`,{className:`hopfield-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{children:`Outer product, then erase self-connections`}),(0,T.jsx)(`h2`,{children:`Turn the saved pattern into weights`})]}),(0,T.jsx)(`code`,{children:`w_ij = p_i p_j / 4, w_ii = 0`})]}),(0,T.jsxs)(`div`,{className:`hopfield-pattern-row`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`stored pattern p`}),(0,T.jsx)(`strong`,{children:Ou(e.storedPattern)})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`normalization`}),(0,T.jsxs)(`strong`,{children:[`divide by `,e.normalization]})]}),(0,T.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`diagonal`}),(0,T.jsx)(`strong`,{children:`set to 0`})]})]}),(0,T.jsxs)(`div`,{className:`hopfield-matrix`,role:`table`,"aria-label":`Hopfield learned weight matrix`,children:[(0,T.jsx)(`div`,{className:`hopfield-matrix__corner`}),e.storedPattern.map((e,t)=>(0,T.jsxs)(`b`,{children:[`from `,t]},`column-${t}`)),e.weights.map((e,t)=>(0,T.jsxs)(`div`,{className:`hopfield-matrix__row`,role:`row`,children:[(0,T.jsxs)(`b`,{children:[`to `,t]}),e.map((e,n)=>(0,T.jsx)(`code`,{className:t===n?`hopfield-weight hopfield-weight--diagonal`:`hopfield-weight`,children:Du(e)},`${t}-${n}`))]},`row-${t}`))]}),(0,T.jsx)(`p`,{className:`hopfield-note`,children:`Symmetry makes the energy score valid. A zero diagonal keeps each neuron from voting for itself.`})]}),(0,T.jsxs)(`section`,{className:`hopfield-recall-panel`,"aria-label":`Hopfield asynchronous recall trace`,children:[(0,T.jsxs)(`div`,{className:`hopfield-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{children:`Use the newest state immediately`}),(0,T.jsx)(`h2`,{children:`Recall one neuron at a time`})]}),(0,T.jsx)(`code`,{children:`state_i = sign(sum_j w_ij state_j)`})]}),(0,T.jsxs)(`div`,{className:`hopfield-recall-lane`,children:[(0,T.jsxs)(`div`,{className:`hopfield-state`,children:[(0,T.jsx)(`small`,{children:`damaged cue`}),(0,T.jsx)(`strong`,{children:Ou(e.corruptedState)}),(0,T.jsxs)(`span`,{children:[`distance `,e.initialHammingDistance]})]}),e.updates.map((e,t)=>(0,T.jsxs)(`div`,{className:r>t?`hopfield-update hopfield-update--visible`:`hopfield-update`,children:[(0,T.jsxs)(`small`,{children:[`update `,e.neuronIndex]}),(0,T.jsx)(`strong`,{children:r>t?Ou(e.stateAfter):`?`}),(0,T.jsx)(`span`,{children:r>t?`field ${Du(e.localField)}`:`advance to reveal`})]},e.step))]}),(0,T.jsxs)(`div`,{className:`hopfield-audit-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`visible state`}),(0,T.jsx)(`strong`,{children:Ou(a)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`normalized overlap`}),(0,T.jsx)(`strong`,{children:Du(s)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`Hopfield energy`}),(0,T.jsx)(`strong`,{children:Du(o)})]})]}),i===null?(0,T.jsx)(`div`,{className:`hopfield-contribution-panel`,children:(0,T.jsx)(`p`,{children:t===0?`The stored pattern is already a low-energy fixed point.`:`The cue matches three of four saved bits. Update neuron 0 first.`})}):(0,T.jsxs)(`div`,{className:`hopfield-contribution-panel`,"aria-label":`Hopfield active neuron calculation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`active neuron`}),(0,T.jsx)(`strong`,{children:i.neuronIndex})]}),(0,T.jsx)(`div`,{className:`hopfield-contributions`,children:i.incoming.map(e=>(0,T.jsxs)(`code`,{children:[Du(e.weight),` x `,e.sourceState>0?`+1`:`-1`,` = `,Du(e.contribution)]},e.sourceIndex))}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`local field -> next state`}),(0,T.jsxs)(`strong`,{children:[Du(i.localField),` -> `,i.nextState>0?`+1`:`-1`]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`energy before -> after`}),(0,T.jsxs)(`strong`,{children:[Du(i.energyBefore),` -> `,Du(i.energyAfter)]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`overlap before -> after`}),(0,T.jsxs)(`strong`,{children:[Du(i.overlapBefore),` -> `,Du(i.overlapAfter)]})]})]})]})]}),(0,T.jsxs)(`aside`,{className:`hopfield-controls`,"aria-label":`Hopfield phase controls`,children:[(0,T.jsx)(`p`,{children:`Associative recall`}),(0,T.jsx)(`h2`,{children:`Advance the memory`}),(0,T.jsx)(`p`,{children:`The first recall step repairs the flipped bit. The other steps prove the recovered pattern is stable under a complete deterministic sweep.`}),(0,T.jsx)(`div`,{className:`hopfield-phase-buttons`,children:ku.map((e,r)=>(0,T.jsxs)(`button`,{"aria-pressed":t===r,type:`button`,onClick:()=>n(r),children:[(0,T.jsx)(`span`,{children:e.eyebrow}),(0,T.jsx)(`strong`,{children:e.title})]},e.title))}),(0,T.jsxs)(`div`,{className:`hopfield-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected state`}),(0,T.jsx)(`strong`,{children:ku[t].title}),(0,T.jsxs)(`span`,{children:[`energy = `,Du(o)]}),(0,T.jsxs)(`span`,{children:[`overlap = `,Du(s)]}),t===ku.length-1?(0,T.jsx)(`b`,{children:`fixed point recovered`}):null]})]})]})}var ju=[1,2,-1],Mu=[{source:0,target:1},{source:1,target:2}],Nu={messageWeight:.5,selfWeight:.25,bias:-.5};function Pu(e){return Math.abs(e)<1e-12?0:e}function Fu(e=ju,t=Mu,n=Nu){let r=[...e,n.messageWeight,n.selfWeight,n.bias];if(e.length<2||!r.every(Number.isFinite)||t.length<1||t.some(t=>!Number.isInteger(t.source)||!Number.isInteger(t.target)||t.source<0||t.target<0||t.source>=e.length||t.target>=e.length||t.source===t.target))throw Error(`NN21 V1 needs finite node features and valid non-self undirected edges.`);let i=t.map(e=>`${Math.min(e.source,e.target)}-${Math.max(e.source,e.target)}`);if(new Set(i).size!==i.length)throw Error(`NN21 V1 needs unique undirected edges.`);let a=t.flatMap(e=>[{source:e.source,target:e.target},{source:e.target,target:e.source}]).map(({source:t,target:r})=>{let i=e[t];return{source:t,target:r,sourceFeature:i,messageWeight:n.messageWeight,message:Pu(n.messageWeight*i)}}).sort((e,t)=>e.target-t.target||e.source-t.source),o=e.map((e,t)=>{let r=a.filter(e=>e.target===t),i=Pu(r.reduce((e,t)=>e+t.message,0)),o=Pu(n.selfWeight*e),s=Pu(o+i+n.bias);return{node:t,oldFeature:e,incoming:r,aggregate:i,selfContribution:o,bias:n.bias,preactivation:s,outputFeature:Math.max(0,s)}});return{nodeFeatures:[...e],edges:t.map(e=>({...e})),parameters:{...n},directedMessages:a,nodeUpdates:o,outputFeatures:o.map(e=>e.outputFeature)}}function Iu(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):e.toFixed(2).replace(/0+$/,``).replace(/\.$/,``)}var Lu=[`Graph`,`Messages`,`Aggregate`,`Update`];function Ru(){let e=(0,l.useMemo)(()=>Fu(),[]),[t,n]=(0,l.useState)(`Graph`),[r,i]=(0,l.useState)(1),a=e.nodeUpdates[r],o=t!==`Graph`,s=t===`Aggregate`||t===`Update`,c=t===`Update`;return(0,T.jsxs)(`main`,{className:`workspace workspace--message-passing`,children:[(0,T.jsxs)(`section`,{className:`message-stage`,"aria-label":`Tiny graph message-passing trace`,children:[(0,T.jsxs)(`div`,{className:`message-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN21 - neighbors send, nodes collect, one round updates`}),(0,T.jsx)(`h2`,{children:`Pass scalar messages across a three-node path`}),(0,T.jsx)(`p`,{children:`Expand two undirected edges into four directed messages, sum each inbox, and update all nodes from the same saved feature snapshot.`})]}),(0,T.jsx)(`div`,{className:`message-chip`,children:`3 nodes - 2 edges`})]}),(0,T.jsxs)(`section`,{className:`message-graph-panel`,"aria-label":`Tiny graph and directed messages`,children:[(0,T.jsxs)(`div`,{className:`message-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{children:`Synchronous round`}),(0,T.jsx)(`h2`,{children:`Original features stay fixed while messages travel`})]}),(0,T.jsx)(`code`,{children:`m(source -> target) = 0.5 x source`})]}),(0,T.jsxs)(`div`,{className:`message-graph`,children:[e.nodeFeatures.map((t,n)=>(0,T.jsxs)(`button`,{className:r===n?`message-node message-node--selected`:`message-node`,type:`button`,onClick:()=>i(n),children:[(0,T.jsxs)(`small`,{children:[`node `,n]}),(0,T.jsx)(`strong`,{children:Iu(c?e.outputFeatures[n]:t)}),(0,T.jsx)(`span`,{children:c?`new feature`:`old feature`})]},n)),(0,T.jsx)(`div`,{className:`message-edge message-edge--left`,children:`0 <-> 1`}),(0,T.jsx)(`div`,{className:`message-edge message-edge--right`,children:`1 <-> 2`})]}),(0,T.jsx)(`div`,{className:`message-ledger`,children:e.directedMessages.map(e=>(0,T.jsxs)(`div`,{className:o&&e.target===r?`message-card message-card--active`:`message-card`,children:[(0,T.jsxs)(`small`,{children:[e.source,` -> `,e.target]}),(0,T.jsxs)(`code`,{children:[`0.5 x `,Iu(e.sourceFeature)]}),(0,T.jsx)(`strong`,{children:o?Iu(e.message):`?`})]},`${e.source}-${e.target}`))})]}),(0,T.jsxs)(`section`,{className:`message-update-panel`,"aria-label":`Selected graph node update`,children:[(0,T.jsxs)(`div`,{className:`message-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{children:[`Selected node `,r]}),(0,T.jsx)(`h2`,{children:`Open its inbox and update equation`})]}),(0,T.jsx)(`code`,{children:`ReLU(0.25 x self + sum(messages) - 0.5)`})]}),(0,T.jsxs)(`div`,{className:`message-inbox`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`incoming messages`}),(0,T.jsx)(`strong`,{children:o?a.incoming.map(e=>Iu(e.message)).join(` + `):`hidden`})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`sum aggregate`}),(0,T.jsx)(`strong`,{children:s?Iu(a.aggregate):`?`})]})]}),(0,T.jsxs)(`div`,{className:`message-equation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`self route`}),(0,T.jsxs)(`code`,{children:[`0.25 x `,Iu(a.oldFeature)]}),(0,T.jsx)(`strong`,{children:s?Iu(a.selfContribution):`?`})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`neighbor route`}),(0,T.jsx)(`code`,{children:`sum inbox`}),(0,T.jsx)(`strong`,{children:s?Iu(a.aggregate):`?`})]}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`bias`}),(0,T.jsx)(`code`,{children:`-0.5`}),(0,T.jsx)(`strong`,{children:s?`-0.5`:`?`})]}),(0,T.jsx)(`span`,{children:`=`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`preactivation`}),(0,T.jsx)(`code`,{children:`before ReLU`}),(0,T.jsx)(`strong`,{children:s?Iu(a.preactivation):`?`})]}),(0,T.jsx)(`span`,{children:`->`}),(0,T.jsxs)(`div`,{className:`message-output`,children:[(0,T.jsx)(`small`,{children:`new feature`}),(0,T.jsx)(`code`,{children:`ReLU`}),(0,T.jsx)(`strong`,{children:c?Iu(a.outputFeature):`?`})]})]}),(0,T.jsx)(`p`,{className:`message-sync-note`,children:"All four messages use the original features `[1, 2, -1]`. No node reads another node's new output during this round."})]})]}),(0,T.jsxs)(`aside`,{className:`message-controls`,"aria-label":`Message-passing phase controls`,children:[(0,T.jsx)(`p`,{children:`One graph round`}),(0,T.jsx)(`h2`,{children:`Reveal the pipeline`}),(0,T.jsx)(`p`,{children:`Select any node, then expose directed messages, its order-invariant sum, and the shared update rule.`}),(0,T.jsx)(`div`,{className:`message-phase-buttons`,children:Lu.map((e,r)=>(0,T.jsxs)(`button`,{"aria-pressed":t===e,type:`button`,onClick:()=>n(e),children:[(0,T.jsxs)(`span`,{children:[r,`. Phase`]}),(0,T.jsx)(`strong`,{children:e})]},e))}),(0,T.jsxs)(`div`,{className:`message-selected-summary`,children:[(0,T.jsx)(`small`,{children:`selected node`}),(0,T.jsx)(`strong`,{children:r}),(0,T.jsxs)(`span`,{children:[`neighbors = `,a.incoming.map(e=>e.source).join(`, `)]}),(0,T.jsxs)(`span`,{children:[`output = `,c?Iu(a.outputFeature):`?`]}),c?(0,T.jsx)(`b`,{children:`round complete`}):null]})]})]})}var zu=[[0,1],[0,1,2],[1,2]],Bu=[1,2,-1];function Vu(e=Bu,t=zu){if(e.length<2||!e.every(Number.isFinite)||t.length!==e.length)throw Error(`NN22 V1 needs finite features and one neighborhood per node.`);t.forEach((t,n)=>{if(t.length<1||new Set(t).size!==t.length||!t.includes(n)||t.some(t=>!Number.isInteger(t)||t<0||t>=e.length))throw Error(`NN22 V1 neighborhoods must be unique valid indices and include self-loops.`)});for(let e=0;e<t.length;e+=1)for(let n of t[e])if(!t[n].includes(e))throw Error(`NN22 V1 neighborhoods must be symmetric.`);let n=t.map(e=>e.length),r=t.map((t,r)=>{let i=t.map(t=>{let i=1/Math.sqrt(n[r]*n[t]);return{source:t,sourceFeature:e[t],sourceDegree:n[t],targetDegree:n[r],coefficient:i,contribution:i*e[t]}}),a=i.reduce((e,t)=>e+t.contribution,0);return{target:r,rows:i,preactivation:a,output:Math.max(0,a)}}),i=t.map((t,n)=>{let r=t.map(t=>e[t]),i=Math.max(...r),a=r.map(e=>Math.exp(e-i)),o=a.reduce((e,t)=>e+t,0),s=t.map((t,n)=>{let s=a[n]/o;return{source:t,sourceFeature:e[t],score:r[n],shiftedScore:r[n]-i,exponential:a[n],attentionWeight:s,contribution:s*e[t]}}),c=s.reduce((e,t)=>e+t.contribution,0);return{target:n,rows:s,maximumScore:i,denominator:o,preactivation:c,output:Math.max(0,c)}});return{features:[...e],neighborhoods:t.map(e=>[...e]),degrees:n,gcn:r,gat:i,gcnOutputs:r.map(e=>e.output),gatOutputs:i.map(e=>e.output)}}function Hu(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):e.toFixed(6).replace(/0+$/,``)}function Uu(){let e=(0,l.useMemo)(()=>Vu(),[]),[t,n]=(0,l.useState)(`gcn`),[r,i]=(0,l.useState)(1),a=e.gcn[r],o=e.gat[r];return(0,T.jsxs)(`main`,{className:`workspace workspace--graph-neighborhood`,children:[(0,T.jsxs)(`section`,{className:`graph-neighborhood-stage`,"aria-label":`Graph convolution and attention trace`,children:[(0,T.jsxs)(`div`,{className:`graph-neighborhood-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN22 - same neighborhood, two weighting rules`}),(0,T.jsx)(`h2`,{children:`Compare graph convolution with graph attention`}),(0,T.jsx)(`p`,{children:`Add self-loops to one three-node path, then inspect fixed degree normalization beside learned softmax attention.`})]}),(0,T.jsx)(`div`,{className:`graph-neighborhood-chip`,children:`GCN vs GAT`})]}),(0,T.jsxs)(`section`,{className:`graph-neighborhood-map`,"aria-label":`Graph neighborhood selector`,children:[(0,T.jsxs)(`div`,{className:`graph-neighborhood-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{children:`Original scalar features`}),(0,T.jsx)(`h2`,{children:`Select a target neighborhood`})]}),(0,T.jsx)(`code`,{children:`0(1) <-> 1(2) <-> 2(-1), plus self-loops`})]}),(0,T.jsx)(`div`,{className:`graph-targets`,children:e.features.map((t,n)=>(0,T.jsxs)(`button`,{"aria-pressed":r===n,type:`button`,onClick:()=>i(n),children:[(0,T.jsxs)(`small`,{children:[`node `,n]}),(0,T.jsx)(`strong`,{children:Hu(t)}),(0,T.jsxs)(`span`,{children:[`degree `,e.degrees[n]]})]},n))}),(0,T.jsxs)(`p`,{children:[`Target `,r,` reads sources [`,e.neighborhoods[r].join(`, `),`]. Both models use exactly this same inbox.`]})]}),t===`gcn`?(0,T.jsxs)(`section`,{className:`graph-model-panel`,"aria-label":`Graph convolution calculation`,children:[(0,T.jsxs)(`div`,{className:`graph-neighborhood-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{children:`Fixed structural weights`}),(0,T.jsx)(`h2`,{children:`Normalize by both endpoint degrees`})]}),(0,T.jsx)(`code`,{children:`coefficient = 1 / sqrt(d_target x d_source)`})]}),(0,T.jsx)(`div`,{className:`graph-row-grid`,children:a.rows.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`source `,e.source]}),(0,T.jsxs)(`code`,{children:[`1 / sqrt(`,e.targetDegree,` x `,e.sourceDegree,`)`]}),(0,T.jsx)(`strong`,{children:Hu(e.coefficient)}),(0,T.jsxs)(`span`,{children:[`x feature `,Hu(e.sourceFeature)]}),(0,T.jsxs)(`b`,{children:[`= `,Hu(e.contribution)]})]},e.source))}),(0,T.jsxs)(`div`,{className:`graph-result`,children:[(0,T.jsx)(`span`,{children:`sum contributions`}),(0,T.jsxs)(`strong`,{children:[a.rows.map(e=>Hu(e.contribution)).join(` + `),` = `,Hu(a.preactivation)]}),(0,T.jsxs)(`b`,{children:[`ReLU -> `,Hu(a.output)]})]})]}):(0,T.jsxs)(`section`,{className:`graph-model-panel`,"aria-label":`Graph attention calculation`,children:[(0,T.jsxs)(`div`,{className:`graph-neighborhood-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{children:`Data-dependent weights`}),(0,T.jsx)(`h2`,{children:`Softmax the source scores inside this inbox`})]}),(0,T.jsx)(`code`,{children:`score = source feature; alpha = stable softmax(score)`})]}),(0,T.jsxs)(`div`,{className:`graph-softmax-summary`,children:[(0,T.jsxs)(`span`,{children:[`row max = `,Hu(o.maximumScore)]}),(0,T.jsxs)(`span`,{children:[`denominator = `,Hu(o.denominator)]}),(0,T.jsxs)(`strong`,{children:[`weights sum = `,Hu(o.rows.reduce((e,t)=>e+t.attentionWeight,0))]})]}),(0,T.jsx)(`div`,{className:`graph-row-grid`,children:o.rows.map(e=>(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`source `,e.source]}),(0,T.jsxs)(`code`,{children:[`score `,Hu(e.score),` - max `,Hu(o.maximumScore),` = `,Hu(e.shiftedScore)]}),(0,T.jsxs)(`span`,{children:[`exp = `,Hu(e.exponential)]}),(0,T.jsxs)(`strong`,{children:[`alpha = `,Hu(e.attentionWeight)]}),(0,T.jsxs)(`b`,{children:[`x `,Hu(e.sourceFeature),` = `,Hu(e.contribution)]})]},e.source))}),(0,T.jsxs)(`div`,{className:`graph-result`,children:[(0,T.jsx)(`span`,{children:`weighted sum`}),(0,T.jsxs)(`strong`,{children:[o.rows.map(e=>Hu(e.contribution)).join(` + `),` = `,Hu(o.preactivation)]}),(0,T.jsxs)(`b`,{children:[`ReLU -> `,Hu(o.output)]})]})]}),(0,T.jsxs)(`section`,{className:`graph-output-panel`,"aria-label":`Graph model output comparison`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`GCN outputs`}),(0,T.jsxs)(`strong`,{children:[`[`,e.gcnOutputs.map(Hu).join(`, `),`]`]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`GAT outputs`}),(0,T.jsxs)(`strong`,{children:[`[`,e.gatOutputs.map(Hu).join(`, `),`]`]})]}),(0,T.jsx)(`p`,{children:`GCN weights depend only on graph degrees. GAT weights change with the node features, even though the edges are unchanged.`})]})]}),(0,T.jsxs)(`aside`,{className:`graph-neighborhood-controls`,"aria-label":`Graph model controls`,children:[(0,T.jsx)(`p`,{children:`Neighborhood model`}),(0,T.jsx)(`h2`,{children:`Switch the weighting rule`}),(0,T.jsx)(`p`,{children:`Keep the target and graph fixed while changing how its inbox is weighted.`}),(0,T.jsxs)(`button`,{"aria-pressed":t===`gcn`,type:`button`,onClick:()=>n(`gcn`),children:[(0,T.jsx)(`span`,{children:`Degree rule`}),(0,T.jsx)(`strong`,{children:`Graph convolution`})]}),(0,T.jsxs)(`button`,{"aria-pressed":t===`gat`,type:`button`,onClick:()=>n(`gat`),children:[(0,T.jsx)(`span`,{children:`Softmax rule`}),(0,T.jsx)(`strong`,{children:`Graph attention`})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`selected target`}),(0,T.jsx)(`strong`,{children:r}),(0,T.jsx)(`span`,{children:t===`gcn`?`structural coefficients`:`feature-dependent attention`})]})]})]})}function Wu(){let[e,t]=(0,l.useState)(`hopfield`);return(0,T.jsxs)(`div`,{className:`structured-workbench`,children:[(0,T.jsxs)(`nav`,{className:`structured-lab-switch`,"aria-label":`Structured and memory learning lab`,children:[(0,T.jsx)(`button`,{"aria-pressed":e===`hopfield`,type:`button`,onClick:()=>t(`hopfield`),children:`Hopfield memory`}),(0,T.jsx)(`button`,{"aria-pressed":e===`message`,type:`button`,onClick:()=>t(`message`),children:`Message passing`}),(0,T.jsx)(`button`,{"aria-pressed":e===`graph-models`,type:`button`,onClick:()=>t(`graph-models`),children:`GCN vs GAT`})]}),e===`hopfield`?(0,T.jsx)(Au,{}):e===`message`?(0,T.jsx)(Ru,{}):(0,T.jsx)(Uu,{})]})}var Gu=4,Ku=8,qu=64,Ju=1e6,Yu=[{id:`outer-grid`,title:`Column + row`,summary:`Both inputs expand along one axis.`,left:{shape:[2,1],values:[1,2]},right:{shape:[1,3],values:[10,20,30]},upstream:{shape:[2,3],values:[1,2,3,4,5,6]}},{id:`row-over-batch`,title:`Matrix + rank-one row`,summary:`Right alignment turns [3] into [1, 3].`,left:{shape:[2,3],values:[1,2,3,4,5,6]},right:{shape:[3],values:[10,20,30]},upstream:{shape:[2,3],values:[1,1,1,1,1,1]}},{id:`scalar-over-matrix`,title:`Scalar + matrix`,summary:`A rank-zero value reaches every output cell.`,left:{shape:[],values:[2]},right:{shape:[2,2],values:[1,2,3,4]},upstream:{shape:[2,2],values:[1,-1,2,-2]}},{id:`incompatible-tail`,title:`Mismatch`,summary:`Trailing dimensions 3 and 2 cannot align.`,left:{shape:[2,3],values:[1,2,3,4,5,6]},right:{shape:[2],values:[10,20]},upstream:null}];function Xu(e){return e.reduce((e,t)=>e*t,1)}function Zu(e,t){if(typeof e!=`object`||!e||!Array.isArray(e.shape)||!Array.isArray(e.values))throw Error(`${t} must contain shape and values arrays`);if(e.shape.length>Gu)throw Error(`${t} shape must contain at most ${Gu} dimensions`);e.shape.forEach(e=>{if(!Number.isInteger(e)||e<=0||e>Ku)throw Error(`${t} dimensions must be positive integers up to ${Ku}`)});let n=Xu(e.shape);if(n>qu||e.values.length!==n)throw Error(`${t} values must match its bounded shape`);if(!e.values.every(e=>Number.isFinite(e)&&Math.abs(e)<=Ju))throw Error(`${t} values must be finite and bounded`)}function Qu(e,t){if(!Number.isFinite(e))throw Error(`${t} must remain finite`);return e}function $u(e){let t=Array(e.length).fill(0),n=1;for(let r=e.length-1;r>=0;--r)t[r]=n,n*=e[r];return t}function ed(e,t){return $u(t).map(t=>{let n=Math.floor(e/t);return e%=t,n})}function td(e,t){return e.reduce((e,n,r)=>e+n*$u(t)[r],0)}function nd(e,t){let n=Math.max(e.length,t.length);return[[...Array(n-e.length).fill(1),...e],[...Array(n-t.length).fill(1),...t]]}function rd(e,t,n){let r=0;return n.forEach(n=>{let i=Qu(e[n.leftFlatIndex]+t[n.rightFlatIndex],`broadcast score output`),a=Qu(n.upstream*i,`broadcast score contribution`);r=Qu(r+a,`broadcast score`)}),r}function id(e,t,n,r=1e-5){if(Zu(e,`left tensor`),Zu(t,`right tensor`),!Number.isFinite(r)||r<1e-12||r>1)throw Error(`finite-difference epsilon must be finite and in [1e-12, 1]`);let[i,a]=nd(e.shape,t.shape),o=[];for(let n=0;n<i.length;n+=1){let r=i[n],s=a[n];if(r!==s&&r!==1&&s!==1)return{compatible:!1,left:e,right:t,upstream:null,paddedLeftShape:i,paddedRightShape:a,mismatchAxis:n,leftDimension:r,rightDimension:s,error:`axis ${n}: dimensions ${r} and ${s} are incompatible`};o.push(Math.max(r,s))}if(n===null)throw Error(`compatible shapes require an upstream tensor`);if(Zu(n,`upstream tensor`),n.shape.length!==o.length||n.shape.some((e,t)=>e!==o[t]))throw Error(`upstream shape must equal output shape [${o.join(`, `)}]`);let s=o.length,c=s-e.shape.length,l=s-t.shape.length,u=[];for(let r=0;r<Xu(o);r+=1){let s=ed(r,o),d=s.map((e,t)=>i[t]===1?0:e),f=s.map((e,t)=>a[t]===1?0:e),p=d.slice(c),m=f.slice(l),h=td(p,e.shape),g=td(m,t.shape),_=e.values[h],v=t.values[g],y=Qu(_+v,`broadcast output`);u.push({outputIndex:s,outputFlatIndex:r,leftIndex:p,leftFlatIndex:h,rightIndex:m,rightFlatIndex:g,leftValue:_,rightValue:v,outputValue:y,upstream:n.values[r]})}let d=Array(e.values.length).fill(0),f=Array(t.values.length).fill(0);u.forEach(e=>{d[e.leftFlatIndex]=Qu(d[e.leftFlatIndex]+e.upstream,`left broadcast gradient`),f[e.rightFlatIndex]=Qu(f[e.rightFlatIndex]+e.upstream,`right broadcast gradient`)});let p=e.values.map((n,i)=>{let a=[...e.values],o=[...e.values];return a[i]+=r,o[i]-=r,Qu((rd(a,t.values,u)-rd(o,t.values,u))/(2*r),`left finite-difference gradient`)}),m=t.values.map((n,i)=>{let a=[...t.values],o=[...t.values];return a[i]+=r,o[i]-=r,Qu((rd(e.values,a,u)-rd(e.values,o,u))/(2*r),`right finite-difference gradient`)}),h=[...d.map((e,t)=>Math.abs(e-p[t])),...f.map((e,t)=>Math.abs(e-m[t]))],g=Qu(Math.max(...h,0),`gradient error`);return{compatible:!0,left:e,right:t,upstream:n,paddedLeftShape:i,paddedRightShape:a,outputShape:o,leftExpandedAxes:i.flatMap((e,t)=>e===1&&o[t]>1?[t]:[]),rightExpandedAxes:a.flatMap((e,t)=>e===1&&o[t]>1?[t]:[]),outputValues:u.map(e=>e.outputValue),mappings:u,leftGradient:d,rightGradient:f,finiteDifferenceLeftGradient:p,finiteDifferenceRightGradient:m,maxGradientAbsoluteError:g}}function ad(e=`outer-grid`){let t=Yu.find(t=>t.id===e);if(t===void 0)throw Error(`unknown tensor broadcasting scenario: ${e}`);return{id:t.id,title:t.title,summary:t.summary,...id(t.left,t.right,t.upstream)}}function od(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function sd(e){return e.length===0?`[] scalar`:`[${e.join(`, `)}]`}function cd(e){return e.length===0?`[]`:`[${e.join(`, `)}]`}function ld(e){return`[${e.map(e=>od(e)).join(`, `)}]`}function ud({trace:e}){let t=e.compatible?-1:e.mismatchAxis;return(0,T.jsxs)(`section`,{className:`tensor-shape-panel`,"aria-label":`Right aligned tensor shapes`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 1 / line up the tail`}),(0,T.jsx)(`h2`,{children:`Compare dimensions from the right`})]}),(0,T.jsx)(`span`,{children:`equal or one`})]}),(0,T.jsxs)(`div`,{className:`tensor-shape-equation`,children:[(0,T.jsx)(`code`,{children:sd(e.left.shape)}),(0,T.jsx)(`span`,{children:`+`}),(0,T.jsx)(`code`,{children:sd(e.right.shape)}),(0,T.jsx)(`span`,{children:`→`}),(0,T.jsx)(`strong`,{children:e.compatible?sd(e.outputShape):`shape error`})]}),(0,T.jsx)(`div`,{className:`tensor-axis-grid`,children:e.paddedLeftShape.map((n,r)=>{let i=e.paddedRightShape[r],a=n===i||n===1||i===1;return(0,T.jsxs)(`div`,{className:r===t?`is-mismatch`:``,children:[(0,T.jsxs)(`small`,{children:[`axis `,r]}),(0,T.jsxs)(`code`,{children:[n,` ↔ `,i]}),(0,T.jsx)(`strong`,{children:a?Math.max(n,i):`stop`}),(0,T.jsx)(`span`,{children:n===i?`same`:a?`expand the 1`:`neither is 1`})]},r)})})]})}function dd({trace:e}){return(0,T.jsxs)(`section`,{className:`tensor-gradient-panel`,"aria-label":`Broadcast gradient reduction`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 4 / reverse the reuse`}),(0,T.jsx)(`h2`,{children:`Copied routes add back together`})]}),(0,T.jsx)(`span`,{children:`sum expanded axes`})]}),(0,T.jsxs)(`div`,{className:`tensor-gradient-grid`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`upstream / output shape `,sd(e.outputShape)]}),(0,T.jsx)(`code`,{children:ld(e.upstream.values)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`left gradient / original shape `,sd(e.left.shape)]}),(0,T.jsx)(`code`,{children:ld(e.leftGradient)}),(0,T.jsxs)(`span`,{children:[`reduce axes `,e.leftExpandedAxes.length?e.leftExpandedAxes.join(`, `):`none`]})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`small`,{children:[`right gradient / original shape `,sd(e.right.shape)]}),(0,T.jsx)(`code`,{children:ld(e.rightGradient)}),(0,T.jsxs)(`span`,{children:[`reduce axes `,e.rightExpandedAxes.length?e.rightExpandedAxes.join(`, `):`none`]})]})]}),(0,T.jsxs)(`div`,{className:`tensor-gradient-audit`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`finite-difference epsilon`}),(0,T.jsx)(`code`,{children:`1e-5`})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`left numerical`}),(0,T.jsx)(`code`,{children:ld(e.finiteDifferenceLeftGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`right numerical`}),(0,T.jsx)(`code`,{children:ld(e.finiteDifferenceRightGradient)})]}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`maximum absolute error`}),(0,T.jsx)(`code`,{children:od(e.maxGradientAbsoluteError)})]})]})]})}function fd(){let[e,t]=(0,l.useState)(`outer-grid`),[n,r]=(0,l.useState)(0),i=(0,l.useMemo)(()=>ad(e),[e]),a=i.compatible?i.mappings[Math.min(n,i.mappings.length-1)]:null,o=i.compatible?i.outputShape.at(-1)??1:1;function s(e){t(e),r(0)}return(0,T.jsxs)(`main`,{className:`workspace workspace--tensor-broadcasting`,children:[(0,T.jsxs)(`section`,{className:`tensor-broadcast-stage`,"aria-label":`Tensor shape and broadcasting visualizer`,children:[(0,T.jsxs)(`div`,{className:`tensor-broadcast-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`NN26 / tensor and autograd bridge`}),(0,T.jsx)(`h2`,{children:`Shape and broadcasting microscope`}),(0,T.jsx)(`p`,{children:`A broadcast does not invent new parameters. It reuses an input coordinate wherever an aligned dimension is one.`})]}),(0,T.jsx)(`div`,{className:`tensor-broadcast-chip`,children:`row-major`})]}),(0,T.jsx)(ud,{trace:i}),i.compatible?(0,T.jsxs)(T.Fragment,{children:[(0,T.jsxs)(`section`,{className:`tensor-output-panel`,"aria-label":`Broadcast output coordinate map`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 2 / reuse coordinates`}),(0,T.jsx)(`h2`,{children:`Open any output cell`})]}),(0,T.jsxs)(`span`,{children:[i.outputValues.length,` row-major cells`]})]}),(0,T.jsx)(`div`,{className:`tensor-output-grid`,style:{"--tensor-columns":o},children:i.mappings.map(e=>(0,T.jsxs)(`button`,{"aria-label":`Open output ${cd(e.outputIndex)} value ${od(e.outputValue)}`,"aria-pressed":e.outputFlatIndex===n,type:`button`,onClick:()=>r(e.outputFlatIndex),children:[(0,T.jsx)(`small`,{children:cd(e.outputIndex)}),(0,T.jsx)(`strong`,{children:od(e.outputValue)})]},e.outputFlatIndex))})]}),(0,T.jsxs)(`section`,{className:`tensor-mapping-panel`,"aria-label":`Selected broadcast index calculation`,children:[(0,T.jsxs)(`div`,{className:`panel-heading`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Step 3 / one hand calculation`}),(0,T.jsxs)(`h2`,{children:[`Output `,cd(a.outputIndex)]})]}),(0,T.jsxs)(`span`,{children:[`flat slot `,a.outputFlatIndex]})]}),(0,T.jsxs)(`div`,{className:`tensor-mapping-equation`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`left source`}),(0,T.jsxs)(`code`,{children:[cd(a.leftIndex),` → `,od(a.leftValue)]}),(0,T.jsx)(`span`,{children:i.leftExpandedAxes.length?`axis ${i.leftExpandedAxes.join(`, `)} reuses this slot`:`no left expansion`})]}),(0,T.jsx)(`strong`,{children:`+`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`right source`}),(0,T.jsxs)(`code`,{children:[cd(a.rightIndex),` → `,od(a.rightValue)]}),(0,T.jsx)(`span`,{children:i.rightExpandedAxes.length?`axis ${i.rightExpandedAxes.join(`, `)} reuses this slot`:`no right expansion`})]}),(0,T.jsx)(`strong`,{children:`=`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`small`,{children:`output`}),(0,T.jsxs)(`code`,{children:[cd(a.outputIndex),` → `,od(a.outputValue)]}),(0,T.jsxs)(`span`,{children:[`upstream gradient `,od(a.upstream)]})]})]})]}),(0,T.jsx)(dd,{trace:i})]}):(0,T.jsxs)(`section`,{className:`tensor-mismatch-panel`,"aria-label":`Broadcast shape mismatch`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Stop before touching the buffers`}),(0,T.jsxs)(`h2`,{children:[`Axis `,i.mismatchAxis,` cannot broadcast`]}),(0,T.jsxs)(`code`,{children:[i.leftDimension,` is not `,i.rightDimension,`, and neither dimension is 1`]}),(0,T.jsxs)(`p`,{children:[i.error,`. A tensor library should reject this deterministically instead of recycling values or reading beyond a buffer.`]})]})]}),(0,T.jsxs)(`aside`,{className:`controls tensor-broadcast-controls`,"aria-label":`Tensor broadcasting scenarios`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Shape presets`}),(0,T.jsx)(`h2`,{children:`Change one alignment rule`}),(0,T.jsx)(`div`,{className:`tensor-scenario-buttons`,children:Yu.map(t=>(0,T.jsxs)(`button`,{"aria-pressed":t.id===e,type:`button`,onClick:()=>s(t.id),children:[(0,T.jsx)(`strong`,{children:t.title}),(0,T.jsxs)(`code`,{children:[sd(t.left.shape),` + `,sd(t.right.shape)]}),(0,T.jsx)(`span`,{children:t.summary})]},t.id))}),(0,T.jsxs)(`div`,{className:`tensor-mental-model`,children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`Keep this picture`}),(0,T.jsx)(`h2`,{children:`Forward reuses. Backward sums.`}),(0,T.jsx)(`p`,{children:`First align the tail. Then replace each compatible one with the other dimension. Every reused route contributes when gradients return.`})]})]})]})}var pd={input:2,target:1,weight:.5,bias:.1,learningRate:.1,activation:`linear`};function md(e,t,n){switch(n){case`linear`:return 1;case`sigmoid`:return t*(1-t);case`tanh`:return 1-t*t;case`relu`:return+(e>0)}}function hd(e){let t=e.input*e.weight,n=t+e.bias,r=f(n,e.activation),i=r-e.target,a=i*i,o=2*i,s=md(n,r,e.activation),c=e.input,l=o*s*c,u=o*s*1,d=e.weight-e.learningRate*l,p=e.bias-e.learningRate*u,m=f(e.input*d+p,e.activation),h=m-e.target;return{...e,weightedInput:t,preActivation:n,prediction:r,error:i,loss:a,lossPredictionDerivative:o,activationDerivative:s,preActivationWeightDerivative:c,preActivationBiasDerivative:1,gradientWeight:l,gradientBias:u,nextWeight:d,nextBias:p,nextPrediction:m,nextLoss:h*h}}function J(e,t=5){return Number.isFinite(e)?Math.abs(e)<1e-12?`0`:Math.abs(e)>=1e3||Math.abs(e)>0&&Math.abs(e)<1e-4?e.toExponential(3):Number(e.toFixed(t)).toString():String(e)}var gd=[{id:`example`,shortLabel:`Example`,title:`Choose one training example`,question:`What information is the neuron trying to connect?`,formula:e=>`x = ${J(e.input)}, target = ${J(e.target)}`,value:e=>`x ${J(e.input)} / target ${J(e.target)}`,explanation:()=>`The input is evidence. The target is the answer we want this one neuron to approach.`},{id:`multiply`,shortLabel:`Multiply`,title:`Scale the input by its weight`,question:`How strongly does this input contribute?`,formula:e=>`${J(e.input)} x ${J(e.weight)} = ${J(e.weightedInput)}`,value:e=>J(e.weightedInput),explanation:e=>`The current weight ${J(e.weight)} turns the input into one weighted contribution.`},{id:`bias`,shortLabel:`Add bias`,title:`Shift the weighted contribution`,question:`What should the neuron predict when its input contribution is zero?`,formula:e=>`${J(e.weightedInput)} + ${J(e.bias)} = ${J(e.preActivation)}`,value:e=>`z = ${J(e.preActivation)}`,explanation:e=>`The bias ${J(e.bias)} shifts the neuron before any activation is applied.`},{id:`activation`,shortLabel:`Activate`,title:`Transform the raw sum`,question:`What range or shape should the output have?`,formula:e=>`${e.activation}(${J(e.preActivation)}) = ${J(e.prediction)}`,value:e=>`prediction ${J(e.prediction)}`,explanation:e=>`The ${e.activation} activation transforms z into the value compared with the target.`},{id:`loss`,shortLabel:`Measure loss`,title:`Turn the mistake into one score`,question:`How wrong is the current prediction?`,formula:e=>`(${J(e.prediction)} - ${J(e.target)})^2 = ${J(e.loss)}`,value:e=>`loss ${J(e.loss)}`,explanation:e=>`The signed error is ${J(e.error)}. Squaring it makes the score positive and magnifies larger mistakes.`},{id:`backprop`,shortLabel:`Backprop`,title:`Assign responsibility with the chain rule`,question:`How much did each parameter contribute to the loss?`,formula:e=>`dL/dw = ${J(e.lossPredictionDerivative)} x ${J(e.activationDerivative)} x ${J(e.input)} = ${J(e.gradientWeight)}`,value:e=>`dw ${J(e.gradientWeight)} / db ${J(e.gradientBias)}`,explanation:()=>`Backpropagation multiplies local derivatives along each path from the loss to a parameter.`},{id:`update`,shortLabel:`Update`,title:`Move the parameters against the gradient`,question:`What small change should reduce the loss?`,formula:e=>`w' = ${J(e.weight)} - ${J(e.learningRate)} x ${J(e.gradientWeight)} = ${J(e.nextWeight)}`,value:e=>`w' ${J(e.nextWeight)} / b' ${J(e.nextBias)}`,explanation:e=>`With the proposed parameters, the loss changes from ${J(e.loss)} to ${J(e.nextLoss)}.`}];function _d(e,t){let n=Number(e);return Number.isFinite(n)?n:t}function vd(){let[e,t]=(0,l.useState)(pd),[n,r]=(0,l.useState)(0),[i,a]=(0,l.useState)(0),o=(0,l.useMemo)(()=>hd(e),[e]),s=gd[n];function c(e,n){t(t=>({...t,[e]:_d(n,t[e])})),r(0)}function u(e){t(t=>({...t,activation:e})),r(0)}function d(){t(e=>{let t=hd(e);return{...e,weight:Number(t.nextWeight.toPrecision(12)),bias:Number(t.nextBias.toPrecision(12))}}),a(e=>e+1),r(0)}function f(){t(pd),r(0),a(0)}return(0,T.jsxs)(`main`,{className:`workspace workspace--microscope`,children:[(0,T.jsxs)(`section`,{className:`microscope-stage`,"aria-label":`Training step microscope`,children:[(0,T.jsxs)(`div`,{className:`lab-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:`One neuron / one example / one update`}),(0,T.jsx)(`h2`,{children:`Training-step microscope`}),(0,T.jsx)(`p`,{children:`Reveal the arithmetic in order. Future phases stay hidden until you reach them.`})]}),(0,T.jsxs)(`div`,{className:`lab-chip`,children:[`update `,i]})]}),(0,T.jsx)(`ol`,{className:`phase-strip`,"aria-label":`Training phases`,children:gd.map((e,t)=>(0,T.jsx)(`li`,{children:(0,T.jsxs)(`button`,{className:`phase-button${t===n?` phase-button--active`:``}${t<n?` phase-button--complete`:``}`,type:`button`,onClick:()=>r(t),"aria-current":t===n?`step`:void 0,children:[(0,T.jsx)(`span`,{children:t+1}),e.shortLabel]})},e.id))}),(0,T.jsxs)(`section`,{className:`microscope-focus`,"aria-live":`polite`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsxs)(`p`,{className:`eyebrow`,children:[`Phase `,n+1,` of `,gd.length]}),(0,T.jsx)(`h2`,{children:s.title}),(0,T.jsx)(`p`,{className:`focus-question`,children:s.question})]}),(0,T.jsx)(`code`,{children:s.formula(o)}),(0,T.jsx)(`p`,{children:s.explanation(o)})]}),(0,T.jsx)(`section`,{className:`signal-pipeline`,"aria-label":`Neuron signal pipeline`,children:gd.map((e,t)=>(0,T.jsxs)(`button`,{className:`signal-node${t===n?` signal-node--active`:``}${t>n?` signal-node--locked`:``}`,type:`button`,onClick:()=>r(t),children:[(0,T.jsx)(`span`,{children:e.shortLabel}),(0,T.jsx)(`strong`,{children:t<=n?e.value(o):`?`})]},e.id))}),s.id===`backprop`&&(0,T.jsxs)(`section`,{className:`derivative-panel`,"aria-label":`Chain rule factors`,children:[(0,T.jsxs)(`div`,{className:`derivative-factor`,children:[(0,T.jsx)(`span`,{children:`Loss slope`}),(0,T.jsxs)(`code`,{children:[`dL/dy = `,J(o.lossPredictionDerivative)]})]}),(0,T.jsx)(`div`,{className:`derivative-times`,"aria-hidden":`true`,children:`x`}),(0,T.jsxs)(`div`,{className:`derivative-factor`,children:[(0,T.jsx)(`span`,{children:`Activation slope`}),(0,T.jsxs)(`code`,{children:[`dy/dz = `,J(o.activationDerivative)]})]}),(0,T.jsx)(`div`,{className:`derivative-times`,"aria-hidden":`true`,children:`x`}),(0,T.jsxs)(`div`,{className:`derivative-factor`,children:[(0,T.jsx)(`span`,{children:`Weight path`}),(0,T.jsxs)(`code`,{children:[`dz/dw = `,J(o.preActivationWeightDerivative)]})]}),(0,T.jsx)(`div`,{className:`derivative-equals`,"aria-hidden":`true`,children:`=`}),(0,T.jsxs)(`div`,{className:`derivative-factor derivative-factor--result`,children:[(0,T.jsx)(`span`,{children:`Weight gradient`}),(0,T.jsxs)(`code`,{children:[`dL/dw = `,J(o.gradientWeight)]})]})]}),s.id===`update`&&(0,T.jsxs)(`section`,{className:`before-after`,"aria-label":`Parameter update result`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:`Before`}),(0,T.jsxs)(`strong`,{children:[`w `,J(o.weight),` / b `,J(o.bias)]}),(0,T.jsxs)(`small`,{children:[`prediction `,J(o.prediction),` / loss `,J(o.loss)]})]}),(0,T.jsx)(`div`,{className:`update-arrow`,"aria-hidden":`true`,children:`->`}),(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`span`,{children:`After proposed update`}),(0,T.jsxs)(`strong`,{children:[`w `,J(o.nextWeight),` / b `,J(o.nextBias)]}),(0,T.jsxs)(`small`,{children:[`prediction `,J(o.nextPrediction),` / loss `,J(o.nextLoss)]})]})]}),(0,T.jsxs)(`div`,{className:`microscope-actions`,children:[(0,T.jsx)(`button`,{type:`button`,disabled:n===0,onClick:()=>r(e=>Math.max(0,e-1)),children:`Previous phase`}),n<gd.length-1?(0,T.jsx)(`button`,{className:`primary-action`,type:`button`,onClick:()=>r(e=>Math.min(gd.length-1,e+1)),children:`Next phase`}):(0,T.jsx)(`button`,{className:`primary-action`,type:`button`,onClick:d,children:`Apply this update`}),(0,T.jsx)(`button`,{type:`button`,onClick:f,children:`Reset example`})]})]}),(0,T.jsxs)(`aside`,{className:`controls microscope-controls`,"aria-label":`Microscope values`,children:[(0,T.jsxs)(`div`,{className:`lesson`,children:[(0,T.jsx)(`span`,{children:`Change one thing`}),(0,T.jsx)(`p`,{children:`Adjust a value, then step forward again and watch where its effect first appears.`})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Input x`}),(0,T.jsx)(`input`,{"aria-label":`Input x`,type:`number`,step:`0.1`,value:e.input,onChange:e=>c(`input`,e.target.value)})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Target`}),(0,T.jsx)(`input`,{"aria-label":`Target`,type:`number`,step:`0.1`,value:e.target,onChange:e=>c(`target`,e.target.value)})]}),(0,T.jsxs)(`div`,{className:`field-grid`,children:[(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Weight w`}),(0,T.jsx)(`input`,{"aria-label":`Weight w`,type:`number`,step:`0.05`,value:e.weight,onChange:e=>c(`weight`,e.target.value)})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Bias b`}),(0,T.jsx)(`input`,{"aria-label":`Bias b`,type:`number`,step:`0.05`,value:e.bias,onChange:e=>c(`bias`,e.target.value)})]})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Activation`}),(0,T.jsxs)(`select`,{"aria-label":`Activation`,value:e.activation,onChange:e=>u(e.target.value),children:[(0,T.jsx)(`option`,{value:`linear`,children:`Identity / linear`}),(0,T.jsx)(`option`,{value:`sigmoid`,children:`Sigmoid`}),(0,T.jsx)(`option`,{value:`tanh`,children:`Tanh`}),(0,T.jsx)(`option`,{value:`relu`,children:`ReLU`})]})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Learning rate`}),(0,T.jsx)(`input`,{"aria-label":`Learning rate`,type:`number`,min:`0.0001`,step:`0.01`,value:e.learningRate,onChange:e=>c(`learningRate`,e.target.value)})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Current prediction`}),(0,T.jsx)(`strong`,{children:J(o.prediction)})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Current loss`}),(0,T.jsx)(`strong`,{children:J(o.loss)})]}),(0,T.jsxs)(`div`,{className:`gradients`,children:[(0,T.jsx)(`span`,{children:`Proposed gradients`}),(0,T.jsxs)(`code`,{children:[`dL/dw = `,J(o.gradientWeight)]}),(0,T.jsxs)(`code`,{children:[`dL/db = `,J(o.gradientBias)]})]})]})]})}var yd={width:720,height:460,padLeft:64,padRight:24,padTop:26,padBottom:52},bd=[`#237a57`,`#2563eb`,`#c2413b`,`#b7791f`,`#6d5bd0`];function xd(e,t=3){return Number.isFinite(e)?Math.abs(e)>=1e3?e.toFixed(0):Math.abs(e)<.01&&e!==0?e.toExponential(2):e.toFixed(t):`0`}function Sd(e,t,n){return Math.min(n,Math.max(t,e))}function Y(e,t){let n=e.points.map(e=>e.x),r=[...e.points.map(e=>e.y),...nc(e.points,t),e.idealModel.weight*Math.min(...n)+e.idealModel.bias,e.idealModel.weight*Math.max(...n)+e.idealModel.bias],i=Math.min(...n),a=Math.max(...n),o=Math.min(...r),s=Math.max(...r),c=Math.max((a-i)*.12,1),l=Math.max((s-o)*.16,1);return{...yd,xMin:i-c,xMax:a+c,yMin:o-l,yMax:s+l}}function Cd(e,t){let n=t.width-t.padLeft-t.padRight;return t.padLeft+(e-t.xMin)/(t.xMax-t.xMin)*n}function wd(e,t){let n=t.height-t.padTop-t.padBottom;return t.padTop+(1-(e-t.yMin)/(t.yMax-t.yMin))*n}function Td(e,t){let n=t.xMin,r=t.xMax,[i,a]=nc([{x:n,y:0},{x:r,y:0}],e);return`M ${Cd(n,t)} ${wd(i??0,t)} L ${Cd(r,t)} ${wd(a??0,t)}`}function Ed(e){if(e.length===0)return``;let t=Math.max(...e.map(e=>e.loss),1),n=e[0].epoch,r=Math.max(e[e.length-1].epoch-n,1);return e.map((e,i)=>{let a=(e.epoch-n)/r*250,o=74-Sd(e.loss/t,0,1)*74;return`${i===0?`M`:`L`} ${a.toFixed(2)} ${o.toFixed(2)}`}).join(` `)}function Dd(e){let t=Array.from({length:81},(e,t)=>-4+t*.1),n=t.map(t=>f(t,e)),r=Math.min(...n,-1),i=Math.max(...n,1);return t.map((e,t)=>{let a=(e+4)/8*250,o=82-(n[t]-r)/(i-r)*82;return`${t===0?`M`:`L`} ${a.toFixed(2)} ${o.toFixed(2)}`}).join(` `)}function Od(e,t,n){return{epoch:t.epoch,loss:rc(e.points,t,n),mae:ic(e.points,t),weight:t.weight,bias:t.bias}}function kd(e,t){return e===void 0?bd[0]:bd[Math.max(t.indexOf(e),0)%bd.length]}function Ad(){let[e,t]=(0,l.useState)(`microscope`),[n,r]=(0,l.useState)(wc[0].id),i=wc.find(e=>e.id===n)??wc[0],[a,o]=(0,l.useState)(`linear`),[s,c]=(0,l.useState)(i.defaultLoss),[u,f]=(0,l.useState)(i.defaultLearningRate),[m,h]=(0,l.useState)(i.initialModel.weight),[g,_]=(0,l.useState)(i.initialModel.bias),[v,y]=(0,l.useState)(i.initialModel),[b,x]=(0,l.useState)([Od(i,i.initialModel,i.defaultLoss)]),[ee,S]=(0,l.useState)(null),[C,te]=(0,l.useState)(!1);(0,l.useEffect)(()=>{c(i.defaultLoss),f(i.defaultLearningRate),h(i.initialModel.weight),_(i.initialModel.bias),y(i.initialModel),S(null),te(!1),x([Od(i,i.initialModel,i.defaultLoss)])},[i]);let ne=(0,l.useMemo)(()=>nc(i.points,v),[v,i.points]),w=(0,l.useMemo)(()=>Y(i,v),[v,i]),re=(0,l.useMemo)(()=>rc(i.points,v,s),[s,v,i.points]),ie=(0,l.useMemo)(()=>ic(i.points,v),[v,i.points]),ae=(0,l.useMemo)(()=>p(a),[a]),oe=(0,l.useMemo)(()=>Array.from(new Set(i.points.map(e=>e.group).filter(e=>e!==void 0))),[i.points]),se=(0,l.useMemo)(()=>Tc.map(e=>({category:e,labs:wc.filter(t=>t.category===e)})),[]);function ce(e){y(e.state),S(e),x(t=>[...t.slice(-159),{epoch:e.state.epoch,loss:e.loss,mae:e.mae,weight:e.state.weight,bias:e.state.bias}])}function le(e){let t=sc(i.points,v,u,s,e),n=t[t.length-1];n!==void 0&&ce(n)}function E(){let e={weight:m,bias:g,epoch:0};y(e),S(null),te(!1),x([Od(i,e,s)])}return(0,l.useEffect)(()=>{if(!C)return;let e=window.setInterval(()=>{y(e=>{let t=oc(i.points,e,u,s);return S(t),x(e=>[...e.slice(-159),{epoch:t.state.epoch,loss:t.loss,mae:t.mae,weight:t.state.weight,bias:t.state.bias}]),t.state})},180);return()=>window.clearInterval(e)},[C,u,s,i.points]),(0,T.jsxs)(`div`,{className:`app`,children:[(0,T.jsxs)(`header`,{className:`app-header`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:e===`microscope`?`No hidden magic`:e===`optimization`?`Trust, then independently verify`:e===`convolution`?`One detector, every position`:e===`image-cnn`?`Channels become features`:e===`residual`?`Deep route, short route`:e===`recurrent`?`Memory becomes an input`:e===`attention`?`Every token asks and matches`:e===`representation`?`Compress, then reconstruct`:e===`structured`?`Structure shapes computation`:e===`deep`?`Scale shapes forward and backward signals`:e===`tensor`?`Forward reuses, backward sums`:e===`autograd`?`Record what ran, then reverse it`:e===`gradient-buffer`?`Backward adds, zero clears`:e===`forward-lowering`?`One graph, two executable IRs`:e===`training-lowering`?`Reverse and update become schedules`:e===`linear`?`100-lab foundation`:`Hidden-layer playground`}),(0,T.jsx)(`h1`,{children:`ML Learning Lab`})]}),(0,T.jsxs)(`div`,{className:`header-actions`,children:[(0,T.jsxs)(`div`,{className:`mode-toggle`,"aria-label":`Workbench mode`,children:[(0,T.jsx)(`button`,{className:e===`microscope`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`microscope`),children:`Microscope`}),(0,T.jsx)(`button`,{className:e===`optimization`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`optimization`),children:`Optimization`}),(0,T.jsx)(`button`,{className:e===`linear`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`linear`),children:`Linear`}),(0,T.jsx)(`button`,{className:e===`hidden`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`hidden`),children:`Hidden Layer`}),(0,T.jsx)(`button`,{className:e===`convolution`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`convolution`),children:`Spatial`}),(0,T.jsx)(`button`,{className:e===`image-cnn`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`image-cnn`),children:`Image CNN`}),(0,T.jsx)(`button`,{className:e===`residual`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`residual`),children:`Residual`}),(0,T.jsx)(`button`,{className:e===`recurrent`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`recurrent`),children:`Recurrent`}),(0,T.jsx)(`button`,{className:e===`attention`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`attention`),children:`Attention`}),(0,T.jsx)(`button`,{className:e===`representation`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`representation`),children:`Representation`}),(0,T.jsx)(`button`,{className:e===`structured`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`structured`),children:`Structured`}),(0,T.jsx)(`button`,{className:e===`deep`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`deep`),children:`Deep Training`}),(0,T.jsx)(`button`,{className:e===`tensor`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`tensor`),children:`Tensor + Autograd`}),(0,T.jsx)(`button`,{className:e===`autograd`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`autograd`),children:`Autograd Graph`}),(0,T.jsx)(`button`,{className:e===`gradient-buffer`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`gradient-buffer`),children:`Grad Buffers`}),(0,T.jsx)(`button`,{className:e===`forward-lowering`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`forward-lowering`),children:`IR Lowering`}),(0,T.jsx)(`button`,{className:e===`training-lowering`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`training-lowering`),children:`Train Lowering`}),(0,T.jsx)(`button`,{className:e===`backend-parity`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`backend-parity`),children:`Backend Parity`})]}),(0,T.jsx)(`div`,{className:`formula`,children:e===`microscope`?(0,T.jsxs)(T.Fragment,{children:[`forward `,`->`,` `,(0,T.jsx)(`strong`,{children:`loss`}),` `,`->`,` gradients `,`->`,` update`]}):e===`optimization`?(0,T.jsxs)(T.Fragment,{children:[`loss surface `,`->`,` `,(0,T.jsx)(`strong`,{children:`gradient check`}),` `,`->`,` batch strategy`]}):e===`convolution`?(0,T.jsxs)(T.Fragment,{children:[`window × `,(0,T.jsx)(`strong`,{children:`shared kernel`}),` `,`->`,` feature`]}):e===`image-cnn`?(0,T.jsxs)(T.Fragment,{children:[`channels `,`->`,` `,(0,T.jsx)(`strong`,{children:`normalize + ReLU`}),` `,`->`,` pool`]}):e===`residual`?(0,T.jsxs)(T.Fragment,{children:[`local layers + `,(0,T.jsx)(`strong`,{children:`identity skip`}),` `,`->`,` wider field`]}):e===`recurrent`?(0,T.jsxs)(T.Fragment,{children:[`input + `,(0,T.jsx)(`strong`,{children:`previous state`}),` `,`->`,` next state`]}):e===`attention`?(0,T.jsxs)(T.Fragment,{children:[`2 heads `,`->`,` `,(0,T.jsx)(`strong`,{children:`join`}),` `,`->`,` add + norm`]}):e===`representation`?(0,T.jsxs)(T.Fragment,{children:[`encode `,`->`,` `,(0,T.jsx)(`strong`,{children:`constrained latent`}),` `,`->`,` reconstruct`]}):e===`structured`?(0,T.jsxs)(T.Fragment,{children:[`connections `,`->`,` `,(0,T.jsx)(`strong`,{children:`shared rule`}),` `,`->`,` updated state`]}):e===`deep`?(0,T.jsxs)(T.Fragment,{children:[`initialize `,`->`,` `,(0,T.jsx)(`strong`,{children:`gradient flow`}),` `,`->`,` stabilize`]}):e===`tensor`?(0,T.jsxs)(T.Fragment,{children:[`align shapes `,`->`,` `,(0,T.jsx)(`strong`,{children:`reuse coordinates`}),` `,`->`,` reduce gradients`]}):e===`autograd`?(0,T.jsxs)(T.Fragment,{children:[`record operations `,`->`,` `,(0,T.jsx)(`strong`,{children:`save values`}),` `,`->`,` reverse graph`]}):e===`gradient-buffer`?(0,T.jsxs)(T.Fragment,{children:[`backward adds `,`->`,` `,(0,T.jsx)(`strong`,{children:`step reads`}),` `,`->`,` zero clears`]}):e===`forward-lowering`?(0,T.jsxs)(T.Fragment,{children:[`graph meaning `,`->`,` `,(0,T.jsx)(`strong`,{children:`NeuralIR schedule`}),` `,`->`,` MatrixIR fusion`]}):e===`training-lowering`?(0,T.jsxs)(T.Fragment,{children:[`saved values `,`->`,` `,(0,T.jsx)(`strong`,{children:`backward IR`}),` `,`->`,` optimizer IR`]}):e===`backend-parity`?(0,T.jsxs)(T.Fragment,{children:[`same graph `,`->`,` `,(0,T.jsx)(`strong`,{children:`CPU · Rust · WebGPU`}),` `,`->`,` equal output`]}):e===`linear`?(0,T.jsxs)(T.Fragment,{children:[`y = `,(0,T.jsx)(`strong`,{children:xd(v.weight)}),`x + `,(0,T.jsx)(`strong`,{children:xd(v.bias)})]}):(0,T.jsxs)(T.Fragment,{children:[`inputs `,`->`,` `,(0,T.jsx)(`strong`,{children:`hidden`}),` `,`->`,` prediction`]})})]})]}),e===`microscope`?(0,T.jsx)(vd,{}):e===`optimization`?(0,T.jsx)(Wc,{}):e===`convolution`?(0,T.jsx)(_r,{}):e===`image-cnn`?(0,T.jsx)(Ys,{}):e===`residual`?(0,T.jsx)(gu,{}):e===`recurrent`?(0,T.jsx)(ml,{}):e===`attention`?(0,T.jsx)(He,{}):e===`representation`?(0,T.jsx)(cu,{}):e===`structured`?(0,T.jsx)(Wu,{}):e===`deep`?(0,T.jsx)(Yr,{}):e===`tensor`?(0,T.jsx)(fd,{}):e===`autograd`?(0,T.jsx)(_i,{}):e===`gradient-buffer`?(0,T.jsx)(oa,{}):e===`forward-lowering`?(0,T.jsx)(Ui,{}):e===`training-lowering`?(0,T.jsx)(Mn,{}):e===`backend-parity`?(0,T.jsx)(nr,{}):e===`hidden`?(0,T.jsx)(Is,{}):(0,T.jsxs)(`main`,{className:`workspace workspace--lab`,children:[(0,T.jsxs)(`nav`,{className:`lab-rail`,"aria-label":`ML lab examples`,children:[(0,T.jsxs)(`div`,{className:`rail-summary`,children:[(0,T.jsx)(`strong`,{children:wc.length}),(0,T.jsx)(`span`,{children:`examples`})]}),se.map(({category:e,labs:t})=>(0,T.jsxs)(`section`,{className:`lab-group`,children:[(0,T.jsx)(`h2`,{children:e}),(0,T.jsx)(`div`,{className:`lab-list`,children:t.map(e=>(0,T.jsxs)(`button`,{className:e.id===i.id?`lab-button lab-button--active`:`lab-button`,type:`button`,onClick:()=>r(e.id),children:[(0,T.jsx)(`span`,{children:e.title}),(0,T.jsx)(`small`,{children:e.source.kind===`local-csv`?`CSV`:`Synthetic`})]},e.id))})]},e))]}),(0,T.jsxs)(`section`,{className:`lab-stage`,"aria-label":`Selected lab`,children:[(0,T.jsxs)(`div`,{className:`lab-intro`,children:[(0,T.jsxs)(`div`,{children:[(0,T.jsx)(`p`,{className:`eyebrow`,children:i.category}),(0,T.jsx)(`h2`,{children:i.title}),(0,T.jsx)(`p`,{children:i.summary})]}),(0,T.jsxs)(`div`,{className:`lab-chip`,children:[i.points.length,` points`]})]}),(0,T.jsxs)(`section`,{className:`chart-panel`,"aria-label":`Training chart`,children:[(0,T.jsxs)(`svg`,{viewBox:`0 0 ${w.width} ${w.height}`,role:`img`,"aria-label":`${i.title} fit chart`,children:[(0,T.jsx)(`rect`,{className:`plot-bg`,x:w.padLeft,y:w.padTop,width:w.width-w.padLeft-w.padRight,height:w.height-w.padTop-w.padBottom}),[0,.25,.5,.75,1].map(e=>{let t=w.xMin+(w.xMax-w.xMin)*e,n=w.yMin+(w.yMax-w.yMin)*e;return(0,T.jsxs)(`g`,{children:[(0,T.jsx)(`line`,{className:`grid-line`,x1:Cd(t,w),x2:Cd(t,w),y1:w.padTop,y2:w.height-w.padBottom}),(0,T.jsx)(`text`,{className:`axis-label`,x:Cd(t,w),y:w.height-20,children:xd(t,1)}),(0,T.jsx)(`line`,{className:`grid-line`,x1:w.padLeft,x2:w.width-w.padRight,y1:wd(n,w),y2:wd(n,w)}),(0,T.jsx)(`text`,{className:`axis-label axis-label--y`,x:w.padLeft-10,y:wd(n,w)+4,children:xd(n,1)})]},e)}),(0,T.jsx)(`path`,{className:`ideal-line`,d:Td(i.idealModel,w)}),(0,T.jsx)(`path`,{className:`model-line`,d:Td(v,w)}),i.points.map((e,t)=>{let n=Cd(e.x,w),r=wd(e.y,w),i=wd(ne[t],w),a=kd(e.group,oe);return(0,T.jsxs)(`g`,{children:[(0,T.jsx)(`line`,{className:`error-line`,x1:n,x2:n,y1:r,y2:i}),(0,T.jsx)(`circle`,{className:`truth-point`,cx:n,cy:r,r:`6`,style:{fill:a}}),(0,T.jsx)(`circle`,{className:`prediction-point`,cx:n,cy:i,r:`5`})]},`${e.x}-${e.y}-${t}`)}),(0,T.jsx)(`text`,{className:`axis-title`,x:w.width/2,y:w.height-5,children:i.xLabel}),(0,T.jsx)(`text`,{className:`axis-title axis-title--y`,x:`20`,y:w.height/2,children:i.yLabel})]}),(0,T.jsxs)(`div`,{className:`legend`,"aria-label":`Chart legend`,children:[(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-dot legend-dot--truth`}),`Actual`]}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-dot legend-dot--prediction`}),`Prediction`]}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-line legend-line--model`}),`Current line`]}),(0,T.jsxs)(`span`,{children:[(0,T.jsx)(`i`,{className:`legend-line legend-line--ideal`}),`Best fit`]})]})]}),(0,T.jsx)(Zo,{model:v,lastStep:ee,learningRate:u,lossKind:s,samplePoint:i.points[0],pointCount:i.points.length})]}),(0,T.jsxs)(`aside`,{className:`controls metrics`,"aria-label":`Training controls and metrics`,children:[(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Loss`}),(0,T.jsxs)(`select`,{value:s,onChange:e=>c(e.target.value),children:[(0,T.jsx)(`option`,{value:`mse`,children:`Mean squared error`}),(0,T.jsx)(`option`,{value:`mae`,children:`Mean absolute error`})]})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Activation preview`}),(0,T.jsx)(`select`,{value:a,onChange:e=>o(e.target.value),children:d.map(e=>(0,T.jsx)(`option`,{value:e.kind,children:e.label},e.kind))})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Learning rate`}),(0,T.jsx)(`input`,{type:`range`,min:i.learningRateMin,max:i.learningRateMax,step:i.learningRateStep,value:u,onChange:e=>f(Number(e.target.value))}),(0,T.jsx)(`input`,{type:`number`,min:i.learningRateMin,max:i.learningRateMax,step:i.learningRateStep,value:u,onChange:e=>f(Number(e.target.value))})]}),(0,T.jsxs)(`div`,{className:`field-grid`,children:[(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Initial weight`}),(0,T.jsx)(`input`,{type:`number`,step:`0.1`,value:m,onChange:e=>h(Number(e.target.value))})]}),(0,T.jsxs)(`label`,{className:`field`,children:[(0,T.jsx)(`span`,{children:`Initial bias`}),(0,T.jsx)(`input`,{type:`number`,step:`0.5`,value:g,onChange:e=>_(Number(e.target.value))})]})]}),(0,T.jsxs)(`div`,{className:`button-grid`,children:[(0,T.jsx)(`button`,{type:`button`,onClick:()=>le(1),children:`Step`}),(0,T.jsx)(`button`,{type:`button`,onClick:()=>le(25),children:`Step 25`}),(0,T.jsx)(`button`,{type:`button`,onClick:()=>te(e=>!e),children:C?`Pause`:`Run`}),(0,T.jsx)(`button`,{type:`button`,onClick:E,children:`Reset`})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Epoch`}),(0,T.jsx)(`strong`,{children:v.epoch})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Loss`}),(0,T.jsx)(`strong`,{children:xd(re,4)})]}),(0,T.jsxs)(`div`,{className:`metric`,children:[(0,T.jsx)(`span`,{children:`Average error`}),(0,T.jsx)(`strong`,{children:xd(ie,3)})]}),(0,T.jsxs)(`div`,{className:`history`,children:[(0,T.jsxs)(`div`,{className:`history__topline`,children:[(0,T.jsx)(`span`,{children:`Loss history`}),(0,T.jsxs)(`strong`,{children:[b.length,` points`]})]}),(0,T.jsxs)(`svg`,{viewBox:`0 0 250 74`,role:`img`,"aria-label":`Loss history sparkline`,children:[(0,T.jsx)(`path`,{className:`history-grid`,d:`M 0 37 L 250 37`}),(0,T.jsx)(`path`,{className:`history-line`,d:Ed(b)})]})]}),(0,T.jsxs)(`div`,{className:`gradients`,children:[(0,T.jsx)(`span`,{children:`Last gradient`}),(0,T.jsxs)(`code`,{children:[`w `,ee===null?`0.000`:xd(ee.gradientWeight,3)]}),(0,T.jsxs)(`code`,{children:[`b `,ee===null?`0.000`:xd(ee.gradientBias,3)]})]}),(0,T.jsxs)(`div`,{className:`lesson`,children:[(0,T.jsx)(`span`,{children:`Learning note`}),(0,T.jsx)(`p`,{children:i.lesson})]}),(0,T.jsxs)(`div`,{className:`activation-panel`,children:[(0,T.jsxs)(`div`,{className:`history__topline`,children:[(0,T.jsx)(`span`,{children:ae.label}),(0,T.jsx)(`strong`,{children:`f(x)`})]}),(0,T.jsxs)(`svg`,{viewBox:`0 0 250 82`,role:`img`,"aria-label":`${ae.label} activation curve`,children:[(0,T.jsx)(`path`,{className:`history-grid`,d:`M 0 41 L 250 41`}),(0,T.jsx)(`path`,{className:`activation-line`,d:Dd(a)})]}),(0,T.jsx)(`p`,{children:ae.summary})]}),(0,T.jsxs)(`div`,{className:`source-panel`,children:[(0,T.jsx)(`span`,{children:`Data source`}),(0,T.jsx)(`p`,{children:i.source.name}),(0,T.jsx)(`code`,{children:i.source.license})]})]})]})]})}function jd(e){return`ruleName`in e}var Md=class extends Error{token;constructor(e,t){super(t?`Parse error at ${t.line}:${t.column}: ${e}`:`Parse error: ${e}`),this.name=`GrammarParseError`,this.token=t??null}},Nd=class{tokens;grammar;pos;rules;ruleIndex;newlinesSignificant;memo;furthestPos;furthestExpected;_preParseHooks=[];_postParseHooks=[];trace;preserveSourceInfo;constructor(e,t,n){this.tokens=e,this.grammar=t,this.pos=0,this.memo=new Map,this.furthestPos=0,this.furthestExpected=[],this.trace=n?.trace??!1,this.preserveSourceInfo=n?.preserveSourceInfo??!1;let r=new Map,i=new Map;for(let e=0;e<t.rules.length;e++){let n=t.rules[e];r.set(n.name,n),i.set(n.name,e)}this.rules=r,this.ruleIndex=i,this.newlinesSignificant=this.grammarReferencesNewline()}isNewlinesSignificant(){return this.newlinesSignificant}addPreParse(e){this._preParseHooks.push(e)}addPostParse(e){this._postParseHooks.push(e)}parse(){if(this._preParseHooks.length>0){let e=[...this.tokens];for(let t of this._preParseHooks)e=t(e);this.tokens=e}if(this.grammar.rules.length===0)throw new Md(`Grammar has no rules`);let e=this.grammar.rules[0],t=this.parseRule(e.name);if(t===null){let e=this.current();if(this.furthestExpected.length>0){let t=this.furthestExpected.join(` or `),n=this.furthestPos<this.tokens.length?this.tokens[this.furthestPos]:e;throw new Md(`Expected ${t}, got ${JSON.stringify(n.value)}`,n)}throw new Md(`Failed to parse`,e)}for(;this.pos<this.tokens.length&&this.current().type===`NEWLINE`;)this.pos++;if(this.pos<this.tokens.length&&this.current().type!==`EOF`){let e=this.current();if(this.furthestExpected.length>0&&this.furthestPos>this.pos){let t=this.furthestExpected.join(` or `),n=this.furthestPos<this.tokens.length?this.tokens[this.furthestPos]:e;throw new Md(`Expected ${t}, got ${JSON.stringify(n.value)}`,n)}throw new Md(`Unexpected token: ${JSON.stringify(e.value)}`,e)}let n=t;for(let e of this._postParseHooks)n=e(n);return n}current(){return this.pos<this.tokens.length?this.tokens[this.pos]:this.tokens[this.tokens.length-1]}recordFailure(e){this.pos>this.furthestPos?(this.furthestPos=this.pos,this.furthestExpected=[e]):this.pos===this.furthestPos&&(this.furthestExpected.includes(e)||this.furthestExpected.push(e))}grammarReferencesNewline(){for(let e of this.grammar.rules)if(this.elementReferencesNewline(e.body))return!0;return!1}elementReferencesNewline(e){switch(e.type){case`token_reference`:return e.name===`NEWLINE`;case`sequence`:return e.elements.some(e=>this.elementReferencesNewline(e));case`alternation`:return e.choices.some(e=>this.elementReferencesNewline(e));case`repetition`:case`optional`:case`group`:case`positive_lookahead`:case`negative_lookahead`:case`one_or_more`:return this.elementReferencesNewline(e.element);case`separated_repetition`:return this.elementReferencesNewline(e.element)||this.elementReferencesNewline(e.separator);default:return!1}}parseRule(e){let t=this.rules.get(e);if(!t)return null;let n=this.ruleIndex.get(e);if(n!==void 0){let t=`${n},${this.pos}`,r=this.memo.get(t);if(r!==void 0)return this.pos=r.endPos,r.ok?this.buildNode(e,r.children):null}let r=this.pos;if(n!==void 0){let e=`${n},${r}`;this.memo.set(e,{children:null,endPos:r,ok:!1})}if(this.trace){let t=this.current();process.stderr.write(`[TRACE] rule '${e}' at token ${r} (${t.type} "${t.value}") → `)}let i=this.matchElement(t.body);if(this.trace&&process.stderr.write(i===null?`fail
`:`match
`),n!==void 0){let e=`${n},${r}`;if(i===null?this.memo.set(e,{children:null,endPos:this.pos,ok:!1}):this.memo.set(e,{children:i,endPos:this.pos,ok:!0}),i!==null)for(;;){let n=this.pos;this.pos=r,this.memo.set(e,{children:i,endPos:n,ok:!0});let a=this.matchElement(t.body);if(a===null||this.pos<=n){this.pos=n,this.memo.set(e,{children:i,endPos:n,ok:!0});break}i=a}}return i===null?(this.pos=r,this.recordFailure(e),null):this.buildNode(e,i)}matchElement(e){let t=this.pos;switch(e.type){case`sequence`:{let n=[];for(let r of e.elements){let e=this.matchElement(r);if(e===null)return this.pos=t,null;n.push(...e)}return n}case`alternation`:for(let n of e.choices){this.pos=t;let e=this.matchElement(n);if(e!==null)return e}return this.pos=t,null;case`repetition`:{let t=[];for(;;){let n=this.pos,r=this.matchElement(e.element);if(r===null){this.pos=n;break}t.push(...r)}return t}case`optional`:{let t=this.matchElement(e.element);return t===null?[]:t}case`group`:return this.matchElement(e.element);case`token_reference`:return this.matchTokenReference(e.name);case`rule_reference`:{let n=this.parseRule(e.name);return n===null?(this.pos=t,null):[n]}case`literal`:{let t=this.current();if(!this.newlinesSignificant)for(;t.type===`NEWLINE`;)this.pos++,t=this.current();return t.value===e.value?(this.pos++,[t]):(this.recordFailure(`"${e.value}"`),null)}case`positive_lookahead`:{let n=this.matchElement(e.element);return this.pos=t,n===null?null:[]}case`negative_lookahead`:{let n=this.matchElement(e.element);return this.pos=t,n===null?[]:null}case`one_or_more`:{let n=this.matchElement(e.element);if(n===null)return this.pos=t,null;let r=[...n];for(;;){let t=this.pos,n=this.matchElement(e.element);if(n===null){this.pos=t;break}r.push(...n)}return r}case`separated_repetition`:{let n=this.matchElement(e.element);if(n===null)return this.pos=t,e.atLeastOne?null:[];let r=[...n];for(;;){let t=this.pos,n=this.matchElement(e.separator);if(n===null){this.pos=t;break}let i=this.matchElement(e.element);if(i===null){this.pos=t;break}r.push(...n,...i)}return r}default:return null}}matchTokenReference(e){let t=this.current();if(!this.newlinesSignificant&&e!==`NEWLINE`)for(;t.type===`NEWLINE`;)this.pos++,t=this.current();return t.type===e?(this.pos++,[t]):(this.recordFailure(e),null)}buildNode(e,t){let n=Pd(t),r=this.preserveSourceInfo?Fd(t):null;return{ruleName:e,children:t,...n??{},...r??{}}}};function Pd(e){let t=Id(e),n=Ld(e);return!t||!n?null:{startLine:t.line,startColumn:t.column,endLine:n.line,endColumn:n.column}}function Fd(e){let t=Id(e),n=Ld(e);if(!t||!n)return null;let r={};return t.startOffset!==void 0&&(r.startOffset=t.startOffset),n.endOffset!==void 0&&(r.endOffset=n.endOffset),t.tokenIndex!==void 0&&(r.firstTokenIndex=t.tokenIndex),n.tokenIndex!==void 0&&(r.lastTokenIndex=n.tokenIndex),t.leadingTrivia!==void 0&&(r.leadingTrivia=t.leadingTrivia),r}function Id(e){for(let t of e)if(jd(t)){let e=Id(t.children);if(e)return e}else return t;return null}function Ld(e){for(let t=e.length-1;t>=0;t--){let n=e[t];if(jd(n)){let e=Ld(n.children);if(e)return e}else return n}return null}var Rd=class extends Error{line;column;constructor(e,t,n){super(`Lexer error at ${t}:${n}: ${e}`),this.name=`LexerError`,this.line=t,this.column=n}};function zd(e){return e.replace(/[.*+?^${}()|[\]\\]/g,`\\$&`)}function Bd(e){return e.replace(/\(\?i:([^()]+)\)/g,(e,t)=>t.replace(/[A-Za-z]/g,e=>`[${e.toLowerCase()}${e.toUpperCase()}]`))}function Vd(e,t){return new RegExp(Bd(e),t)}function Hd(e,t,n,r,i,a,o){if(e===`NAME`&&r.has(t))throw new Rd(`Reserved keyword '${t}' cannot be used as an identifier`,a,o);return e===`NAME`&&n.has(t)?`KEYWORD`:i||e}function Ud(e){let t=[],n=0;for(;n<e.length;)if(e[n]===`\\`&&n+1<e.length){let r={n:`
`,t:`	`,"\\":`\\`,'"':`"`},i=e[n+1];t.push(r[i]??i),n+=2}else t.push(e[n]),n+=1;return t.join(``)}var Wd=class{_lexer;_source;_posAfter;_suppressed=!1;_emitted=[];_groupActions=[];_skipEnabled=null;_previousToken;_currentTokenLine;constructor(e,t,n,r,i){this._lexer=e,this._source=t,this._posAfter=n,this._previousToken=r,this._currentTokenLine=i}pushGroup(e){if(!this._lexer.hasGroup(e))throw Error(`Unknown pattern group: '${e}'. Available groups: ${this._lexer.availableGroups().sort().join(`, `)}`);this._groupActions.push([`push`,e])}popGroup(){this._groupActions.push([`pop`,``])}activeGroup(){return this._lexer.activeGroup()}groupStackDepth(){return this._lexer.groupStackDepth()}emit(e){this._emitted.push(e)}suppress(){this._suppressed=!0}peek(e=1){let t=this._posAfter+e-1;return t>=0&&t<this._source.length?this._source[t]:``}peekStr(e){return this._source.slice(this._posAfter,this._posAfter+e)}setSkipEnabled(e){this._skipEnabled=e}previousToken(){return this._previousToken}bracketDepth(e){return this._lexer.bracketDepth(e)}precededByNewline(){return this._previousToken===null?!1:this._previousToken.line<this._currentTokenLine}},Gd=class{_source;_pos=0;_line=1;_column=1;_grammar;_keywordSet;_reservedSet;_hasSkipPatterns;_indentationMode;_layoutMode;_caseSensitive;_caseInsensitive;_patterns;_skipPatterns;_groupPatterns;_aliasMap;_groupStack=[`default`];_transitions;_startMode;_inheritingModes;_onToken=null;_skipEnabled=!0;_lastEmittedToken=null;_bracketDepths={paren:0,bracket:0,brace:0};_contextKeywordSet;_layoutKeywordSet;_preTokenizeHooks=[];_postTokenizeHooks=[];_preserveSourceInfo;_pendingTrivia=[];_nextTokenIndex=0;constructor(e,t,n){this._grammar=t,this._preserveSourceInfo=n?.preserveSourceInfo===!0,this._caseInsensitive=t.caseInsensitive===!0,this._caseSensitive=t.caseSensitive!==!1&&!this._caseInsensitive,this._source=!this._caseSensitive&&!this._caseInsensitive?e.toLowerCase():e,this._keywordSet=new Set(this._caseInsensitive?t.keywords.map(e=>e.toUpperCase()):t.keywords),this._reservedSet=new Set(t.reservedKeywords??[]),this._contextKeywordSet=new Set(t.contextKeywords??[]),this._indentationMode=t.mode===`indentation`,this._layoutMode=t.mode===`layout`,this._layoutKeywordSet=new Set(t.layoutKeywords??[]),this._hasSkipPatterns=(t.skipDefinitions??[]).length>0,this._aliasMap={};for(let e of t.definitions)e.alias&&(this._aliasMap[e.name]=e.alias);let r=this._caseInsensitive?`i`:``;if(this._patterns=t.definitions.map(e=>{let t=e.isRegex?e.pattern:zd(e.pattern);return{name:e.name,pattern:Vd(t,r),alias:e.alias}}),this._skipPatterns=(t.skipDefinitions??[]).map(e=>{let t=e.isRegex?e.pattern:zd(e.pattern);return{name:e.name,pattern:Vd(t,r)}}),this._groupPatterns={default:[...this._patterns]},t.groups)for(let[e,n]of Object.entries(t.groups)){let t=n.definitions.map(e=>{let t=e.isRegex?e.pattern:zd(e.pattern);return e.alias&&(this._aliasMap[e.name]=e.alias),{name:e.name,pattern:Vd(t,r),alias:e.alias}});this._groupPatterns[e]=t}this._transitions=t.transitions??[];let i=t.startMode;this._startMode=i!==void 0&&(i==="default"||Object.prototype.hasOwnProperty.call(this._groupPatterns,i))?i:`default`;let a=new Set,o=new Set;for(let e of this._transitions)for(let t of e.actions)t.target!==void 0&&(t.kind===`push`&&a.add(t.target),t.kind===`set_mode`&&o.add(t.target));let s=new Set;for(let e of o)e!=="default"&&!a.has(e)&&s.add(e);this._inheritingModes=s,this._groupStack=[this._startMode]}setOnToken(e){this._onToken=e}hasGroup(e){return e in this._groupPatterns}availableGroups(){return Object.keys(this._groupPatterns)}activeGroup(){return this._groupStack[this._groupStack.length-1]}groupStackDepth(){return this._groupStack.length}bracketDepth(e){return e===void 0?this._bracketDepths.paren+this._bracketDepths.bracket+this._bracketDepths.brace:this._bracketDepths[e]}addPreTokenize(e){this._preTokenizeHooks.push(e)}addPostTokenize(e){this._postTokenizeHooks.push(e)}tokenize(){if(this._preTokenizeHooks.length>0){let e=this._source;for(let t of this._preTokenizeHooks)e=t(e);this._source=e}this._lastEmittedToken=null,this._bracketDepths={paren:0,bracket:0,brace:0},this._pendingTrivia=[],this._nextTokenIndex=0;let e;e=this._indentationMode?this._tokenizeIndentation():this._layoutMode?this._tokenizeLayout():this._tokenizeStandard();for(let t of this._postTokenizeHooks)e=t(e);return e}_tokenizeStandard(){let e=[];for(;this._pos<this._source.length;){let t=this._source[this._pos];if(this._hasSkipPatterns){if(this._skipEnabled&&this._trySkip())continue}else if(t===` `||t===`	`||t===`\r`){this._consumeDefaultWhitespace();continue}if(t===`
`){let t={type:`NEWLINE`,value:`\\n`,line:this._line,column:this._column},n=this._pos;this._advance(),this._emitToken(e,this._withOptionalSourceInfo(t,n));continue}let n=this._groupStack[this._groupStack.length-1],r=this._tryMatchTokenInGroup(n);if(r!==null){if(this._updateBracketDepth(r.value),this._onToken!==null){let t=new Wd(this,this._source,this._pos,this._lastEmittedToken,r.line);this._onToken(r,t),t._suppressed||this._emitToken(e,r);for(let n of t._emitted)this._emitToken(e,n);for(let[e,n]of t._groupActions)e===`push`?this._groupStack.push(n):e===`pop`&&this._groupStack.length>1&&this._groupStack.pop();t._skipEnabled!==null&&(this._skipEnabled=t._skipEnabled),this._applyTransitions(r)}else this._emitToken(e,r),this._applyTransitions(r);continue}throw new Rd(`Unexpected character: ${JSON.stringify(t)}`,this._line,this._column)}let t={type:`EOF`,value:``,line:this._line,column:this._column};return this._emitToken(e,this._withOptionalSourceInfo(t,this._pos)),this._groupStack=[this._startMode],this._skipEnabled=!0,e}_updateBracketDepth(e){if(e.length===1)switch(e){case`(`:this._bracketDepths.paren++;break;case`)`:this._bracketDepths.paren>0&&this._bracketDepths.paren--;break;case`[`:this._bracketDepths.bracket++;break;case`]`:this._bracketDepths.bracket>0&&this._bracketDepths.bracket--;break;case`{`:this._bracketDepths.brace++;break;case`}`:this._bracketDepths.brace>0&&this._bracketDepths.brace--;break}}_transitionKey(e){return e.type}_applyTransitions(e){if(this._transitions.length===0)return;let t=this._transitionKey(e),n=this._groupStack[this._groupStack.length-1]??`default`,r=null;for(let i of this._transitions)if(i.onTokens.includes(t)&&!(i.inMode!==void 0&&i.inMode!==n)&&!(i.onValue!==void 0&&i.onValue!==e.value)){r=i.actions;break}if(r!==null)for(let e of r)switch(e.kind){case`set_mode`:e.target!==void 0&&(this._groupStack[this._groupStack.length-1]=e.target);break;case`push`:e.target!==void 0&&this._groupStack.push(e.target);break;case`pop`:this._groupStack.length>1&&this._groupStack.pop();break;case`enable_skip`:this._skipEnabled=!0;break;case`disable_skip`:this._skipEnabled=!1;break}}_tokenizeIndentation(){let e=[],t=[0],n=0,r=!0;for(;this._pos<this._source.length;){if(r&&n===0){let n=this._processLineStart(t);if(n===`skip`)continue;for(let t of n)this._emitToken(e,t);if(r=!1,this._pos>=this._source.length)break}let i=this._source[this._pos];if(i===`
`){if(n===0){let t={type:`NEWLINE`,value:`\\n`,line:this._line,column:this._column},n=this._pos;this._advance(),this._emitToken(e,this._withOptionalSourceInfo(t,n))}else this._advance();r=!0;continue}if(n>0&&(i===` `||i===`	`||i===`\r`)){this._consumeDefaultWhitespace();continue}if(this._trySkip())continue;let a=this._tryMatchTokenInGroup(`default`);if(a!==null){a.value===`(`||a.value===`[`||a.value===`{`?n++:(a.value===`)`||a.value===`]`||a.value===`}`)&&n--,this._updateBracketDepth(a.value),this._emitToken(e,a),this._applyTransitions(a);continue}throw new Rd(`Unexpected character: ${JSON.stringify(i)}`,this._line,this._column)}for(;t.length>1;)t.pop(),this._emitToken(e,this._withOptionalSourceInfo({type:`DEDENT`,value:``,line:this._line,column:this._column},this._pos));return(e.length===0||e[e.length-1].type!==`NEWLINE`)&&this._emitToken(e,this._withOptionalSourceInfo({type:`NEWLINE`,value:`\\n`,line:this._line,column:this._column},this._pos)),this._emitToken(e,this._withOptionalSourceInfo({type:`EOF`,value:``,line:this._line,column:this._column},this._pos)),this._groupStack=[this._startMode],this._skipEnabled=!0,e}_tokenizeLayout(){return this._applyLayout(this._tokenizeStandard())}_applyLayout(e){let t=[],n=[],r=0,i=0;for(let a=0;a<e.length;a++){let o=e[a],s=o.typeName??o.type;if(s===`NEWLINE`){t.push(o);let r=this._nextLayoutToken(e,a+1);if(i===0&&r!==null){for(;n.length>0&&r.column<n[n.length-1];)t.push(this._virtualLayoutToken(`VIRTUAL_RBRACE`,`}`,r)),n.pop();n.length>0&&(r.typeName??r.type)!==`EOF`&&r.value!==`}`&&r.column===n[n.length-1]&&t.push(this._virtualLayoutToken(`VIRTUAL_SEMICOLON`,`;`,r))}continue}if(s===`EOF`){for(;n.length>0;)t.push(this._virtualLayoutToken(`VIRTUAL_RBRACE`,`}`,o)),n.pop();t.push(o);continue}if(r>0)if(o.value===`{`)--r;else{for(let e=0;e<r;e++)n.push(o.column),t.push(this._virtualLayoutToken(`VIRTUAL_LBRACE`,`{`,o));r=0}t.push(o),this._isVirtualLayoutToken(o)||(o.value===`(`||o.value===`[`||o.value===`{`?i+=1:(o.value===`)`||o.value===`]`||o.value===`}`)&&i>0&&--i),this._isLayoutKeyword(o)&&(r+=1)}return t}_nextLayoutToken(e,t){for(let n=t;n<e.length;n++){let t=e[n];if((t.typeName??t.type)!==`NEWLINE`)return t}return null}_virtualLayoutToken(e,t,n){return this._withOptionalSourceInfo({type:e,typeName:e,value:t,line:n.line,column:n.column},n.startOffset??this._pos)}_isVirtualLayoutToken(e){return(e.typeName??e.type).startsWith(`VIRTUAL_`)}_isLayoutKeyword(e){if(this._layoutKeywordSet.size===0)return!1;let t=e.value??``;return this._layoutKeywordSet.has(t)||this._layoutKeywordSet.has(t.toLowerCase())}_processLineStart(e){let t=0,n=this._line,r=this._column,i=this._pos;for(;this._pos<this._source.length;){let e=this._source[this._pos];if(e===` `)t++,this._advance();else if(e===`	`)throw new Rd(`Tab character in indentation (use spaces only)`,this._line,this._column);else break}if(t>0&&this._preserveSourceInfo&&this._pushTrivia(`WHITESPACE`,this._source.slice(i,this._pos),n,r,i),this._pos>=this._source.length)return`skip`;if(this._source[this._pos]===`
`){let e=this._line,t=this._column,n=this._pos;return this._advance(),this._pushTrivia(`NEWLINE`,`
`,e,t,n),`skip`}let a=this._source.slice(this._pos);for(let e of this._skipPatterns){let t=e.pattern.exec(a);if(t!==null&&t.index===0){let n=this._pos+t[0].length;if(n>=this._source.length||this._source[n]===`
`){let n=this._line,r=this._column,i=this._pos;for(let e=0;e<t[0].length;e++)this._advance();if(this._pushTrivia(e.name,t[0],n,r,i),this._pos<this._source.length&&this._source[this._pos]===`
`){let e=this._line,t=this._column,n=this._pos;this._advance(),this._pushTrivia(`NEWLINE`,`
`,e,t,n)}return`skip`}}}let o=e[e.length-1],s=[];if(t>o)e.push(t),s.push(this._withOptionalSourceInfo({type:`INDENT`,value:``,line:this._line,column:1},this._pos));else if(t<o){for(;e.length>1&&e[e.length-1]>t;)e.pop(),s.push(this._withOptionalSourceInfo({type:`DEDENT`,value:``,line:this._line,column:1},this._pos));if(e[e.length-1]!==t)throw new Rd(`Inconsistent dedent`,this._line,this._column)}return s}_trySkip(){let e=this._source.slice(this._pos);for(let t of this._skipPatterns){let n=t.pattern.exec(e);if(n!==null&&n.index===0){let e=this._line,r=this._column,i=this._pos;for(let e=0;e<n[0].length;e++)this._advance();return this._pushTrivia(t.name,n[0],e,r,i),!0}}return!1}_tryMatchTokenInGroup(e){let t=this._source.slice(this._pos),n=Object.prototype.hasOwnProperty.call(this._groupPatterns,e)?this._groupPatterns[e]:this._patterns;if(e!=="default"&&this._inheritingModes.has(e)){let e=Object.prototype.hasOwnProperty.call(this._groupPatterns,`default`)?this._groupPatterns.default:this._patterns;n=n.concat(e)}for(let{name:e,pattern:r,alias:i}of n){let n=r.exec(t);if(n!==null&&n.index===0){let t=n[0],r=this._line,a=this._column,o=this._pos,s=this._caseInsensitive?t.toUpperCase():t,c=Hd(e,s,this._keywordSet,this._reservedSet,i,r,a);if(this._caseInsensitive&&c===`KEYWORD`&&(t=s),(this._aliasMap[e]??e)===`STRING`||e===`STRING`||e.includes(`STRING`)||i&&i.includes(`STRING`)){if(t.length>=6&&(t.startsWith(`"""`)||t.startsWith(`'''`))){let e=t.slice(3,-3);t=this._grammar.escapeMode===`none`?e:Ud(e)}else if(t.length>=2&&(t[0]===`"`||t[0]===`'`)){let e=t.slice(1,-1);t=this._grammar.escapeMode===`none`?e:Ud(e)}}let l;c===`NAME`&&this._contextKeywordSet.size>0&&this._contextKeywordSet.has(t)&&(l=2);let u=l===void 0?{type:c,value:t,line:r,column:a}:{type:c,value:t,line:r,column:a,flags:l};for(let e=0;e<n[0].length;e++)this._advance();return this._withOptionalSourceInfo(u,o)}}return null}_consumeDefaultWhitespace(){let e=this._line,t=this._column,n=this._pos;for(;this._pos<this._source.length;){let e=this._source[this._pos];if(e!==` `&&e!==`	`&&e!==`\r`)break;this._advance()}this._pos>n&&this._pushTrivia(`WHITESPACE`,this._source.slice(n,this._pos),e,t,n)}_pushTrivia(e,t,n,r,i){this._preserveSourceInfo&&this._pendingTrivia.push({type:e,value:t,line:n,column:r,endLine:this._line,endColumn:this._column,startOffset:i,endOffset:this._pos})}_withOptionalSourceInfo(e,t){return this._preserveSourceInfo?{...e,startOffset:t,endOffset:this._pos,endLine:this._line,endColumn:this._column}:e}_emitToken(e,t){let n=t;this._preserveSourceInfo&&(n={...t,tokenIndex:this._nextTokenIndex++,...this._pendingTrivia.length>0?{leadingTrivia:[...this._pendingTrivia]}:{}},this._pendingTrivia=[]),e.push(n),this._lastEmittedToken=n}_advance(){this._pos<this._source.length&&(this._source[this._pos]===`
`?(this._line+=1,this._column=1):this._column+=1,this._pos+=1)}};function Kd(e,t,n){return new Gd(e,t,n).tokenize()}var qd={version:1,caseInsensitive:!1,caseSensitive:!0,definitions:[{name:`STRING_DQ`,pattern:`"([^"\\\\\\n]|\\\\.)*"`,isRegex:!0,lineNumber:66,alias:`STRING`},{name:`STRING_SQ`,pattern:`'([^'\\\\\\n]|\\\\.)*'`,isRegex:!0,lineNumber:67,alias:`STRING`},{name:`VARIABLE`,pattern:`\\$[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:83},{name:`PLACEHOLDER`,pattern:`%[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:93},{name:`DIMENSION`,pattern:`-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+`,isRegex:!0,lineNumber:102},{name:`PERCENTAGE`,pattern:`-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?%`,isRegex:!0,lineNumber:103},{name:`NUMBER`,pattern:`-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?`,isRegex:!0,lineNumber:104},{name:`HASH`,pattern:`#[a-zA-Z0-9_-]+`,isRegex:!0,lineNumber:110},{name:`AT_KEYWORD`,pattern:`@-?[a-zA-Z][a-zA-Z0-9-]*`,isRegex:!0,lineNumber:127},{name:`URL_TOKEN`,pattern:`url\\([^)'"]*\\)`,isRegex:!0,lineNumber:133},{name:`FUNCTION`,pattern:`-?[a-zA-Z_][a-zA-Z0-9_-]*\\(`,isRegex:!0,lineNumber:139},{name:`CDO`,pattern:`<!--`,isRegex:!1,lineNumber:145},{name:`CDC`,pattern:`-->`,isRegex:!1,lineNumber:146},{name:`UNICODE_RANGE`,pattern:`[Uu]\\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?`,isRegex:!0,lineNumber:152},{name:`CUSTOM_PROPERTY`,pattern:`--[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:153},{name:`IDENT`,pattern:`-?[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:154},{name:`COLON_COLON`,pattern:`::`,isRegex:!1,lineNumber:163},{name:`TILDE_EQUALS`,pattern:`~=`,isRegex:!1,lineNumber:164},{name:`PIPE_EQUALS`,pattern:`|=`,isRegex:!1,lineNumber:165},{name:`CARET_EQUALS`,pattern:`^=`,isRegex:!1,lineNumber:166},{name:`DOLLAR_EQUALS`,pattern:`$=`,isRegex:!1,lineNumber:167},{name:`STAR_EQUALS`,pattern:`*=`,isRegex:!1,lineNumber:168},{name:`EQUALS_EQUALS`,pattern:`==`,isRegex:!1,lineNumber:171},{name:`NOT_EQUALS`,pattern:`!=`,isRegex:!1,lineNumber:172},{name:`GREATER_EQUALS`,pattern:`>=`,isRegex:!1,lineNumber:173},{name:`LESS_EQUALS`,pattern:`<=`,isRegex:!1,lineNumber:174},{name:`LBRACE`,pattern:`{`,isRegex:!1,lineNumber:180},{name:`RBRACE`,pattern:`}`,isRegex:!1,lineNumber:181},{name:`LPAREN`,pattern:`(`,isRegex:!1,lineNumber:182},{name:`RPAREN`,pattern:`)`,isRegex:!1,lineNumber:183},{name:`LBRACKET`,pattern:`[`,isRegex:!1,lineNumber:184},{name:`RBRACKET`,pattern:`]`,isRegex:!1,lineNumber:185},{name:`SEMICOLON`,pattern:`;`,isRegex:!1,lineNumber:186},{name:`COLON`,pattern:`:`,isRegex:!1,lineNumber:187},{name:`COMMA`,pattern:`,`,isRegex:!1,lineNumber:188},{name:`DOT`,pattern:`.`,isRegex:!1,lineNumber:189},{name:`PLUS`,pattern:`+`,isRegex:!1,lineNumber:190},{name:`GREATER`,pattern:`>`,isRegex:!1,lineNumber:191},{name:`LESS`,pattern:`<`,isRegex:!1,lineNumber:192},{name:`TILDE`,pattern:`~`,isRegex:!1,lineNumber:193},{name:`STAR`,pattern:`*`,isRegex:!1,lineNumber:194},{name:`PIPE`,pattern:`|`,isRegex:!1,lineNumber:195},{name:`BANG_DEFAULT`,pattern:`!default`,isRegex:!1,lineNumber:198},{name:`BANG_GLOBAL`,pattern:`!global`,isRegex:!1,lineNumber:199},{name:`BANG`,pattern:`!`,isRegex:!1,lineNumber:200},{name:`SLASH`,pattern:`/`,isRegex:!1,lineNumber:201},{name:`EQUALS`,pattern:`=`,isRegex:!1,lineNumber:202},{name:`AMPERSAND`,pattern:`&`,isRegex:!1,lineNumber:203},{name:`MINUS`,pattern:`-`,isRegex:!1,lineNumber:204}],keywords:[],mode:void 0,escapeMode:`none`,skipDefinitions:[{name:`LINE_COMMENT`,pattern:`\\/\\/[^\\n]*`,isRegex:!0,lineNumber:55},{name:`COMMENT`,pattern:`\\/\\*[\\s\\S]*?\\*\\/`,isRegex:!0,lineNumber:56},{name:`WHITESPACE`,pattern:`[ \\t\\r\\n]+`,isRegex:!0,lineNumber:57}],reservedKeywords:[],layoutKeywords:[],contextKeywords:[],errorDefinitions:[],groups:{}};function Jd(e){return Kd(e,qd)}var Yd={version:1,rules:[{name:`stylesheet`,body:{type:`repetition`,element:{type:`rule_reference`,name:`rule`}},lineNumber:37},{name:`rule`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`lattice_rule`},{type:`rule_reference`,name:`at_rule`},{type:`rule_reference`,name:`qualified_rule`}]},lineNumber:39},{name:`lattice_rule`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`variable_declaration`},{type:`rule_reference`,name:`mixin_definition`},{type:`rule_reference`,name:`function_definition`},{type:`rule_reference`,name:`use_directive`},{type:`rule_reference`,name:`lattice_control`}]},lineNumber:51},{name:`variable_declaration`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`value_list`},{type:`optional`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`BANG_DEFAULT`},{type:`token_reference`,name:`BANG_GLOBAL`}]}},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:69},{name:`mixin_definition`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`literal`,value:`@mixin`},{type:`token_reference`,name:`FUNCTION`},{type:`optional`,element:{type:`rule_reference`,name:`mixin_params`}},{type:`token_reference`,name:`RPAREN`},{type:`rule_reference`,name:`block`}]},{type:`sequence`,elements:[{type:`literal`,value:`@mixin`},{type:`token_reference`,name:`IDENT`},{type:`rule_reference`,name:`block`}]}]},lineNumber:102},{name:`mixin_params`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`mixin_param`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`mixin_param`}]}}]},lineNumber:105},{name:`mixin_param`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`VARIABLE`},{type:`optional`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`mixin_value_list`}]}}]},lineNumber:112},{name:`mixin_value_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`mixin_value`},{type:`repetition`,element:{type:`rule_reference`,name:`mixin_value`}}]},lineNumber:117},{name:`mixin_value`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`UNICODE_RANGE`},{type:`rule_reference`,name:`function_call`},{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`}]},lineNumber:119},{name:`include_directive`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`literal`,value:`@include`},{type:`token_reference`,name:`FUNCTION`},{type:`optional`,element:{type:`rule_reference`,name:`include_args`}},{type:`token_reference`,name:`RPAREN`},{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`SEMICOLON`},{type:`rule_reference`,name:`block`}]}}]},{type:`sequence`,elements:[{type:`literal`,value:`@include`},{type:`token_reference`,name:`IDENT`},{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`SEMICOLON`},{type:`rule_reference`,name:`block`}]}}]}]},lineNumber:130},{name:`include_args`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`include_arg`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`include_arg`}]}}]},lineNumber:133},{name:`include_arg`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`value_list`}]},{type:`rule_reference`,name:`value_list`}]},lineNumber:137},{name:`lattice_control`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`if_directive`},{type:`rule_reference`,name:`for_directive`},{type:`rule_reference`,name:`each_directive`},{type:`rule_reference`,name:`while_directive`}]},lineNumber:160},{name:`if_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@if`},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`literal`,value:`@else`},{type:`literal`,value:`if`},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`}]}},{type:`optional`,element:{type:`sequence`,elements:[{type:`literal`,value:`@else`},{type:`rule_reference`,name:`block`}]}}]},lineNumber:164},{name:`for_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@for`},{type:`token_reference`,name:`VARIABLE`},{type:`literal`,value:`from`},{type:`rule_reference`,name:`lattice_expression`},{type:`group`,element:{type:`alternation`,choices:[{type:`literal`,value:`through`},{type:`literal`,value:`to`}]}},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`}]},lineNumber:171},{name:`each_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@each`},{type:`token_reference`,name:`VARIABLE`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`VARIABLE`}]}},{type:`literal`,value:`in`},{type:`rule_reference`,name:`each_list`},{type:`rule_reference`,name:`block`}]},lineNumber:176},{name:`each_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`value`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`value`}]}}]},lineNumber:179},{name:`while_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@while`},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`}]},lineNumber:184},{name:`lattice_expression`,body:{type:`rule_reference`,name:`lattice_or_expr`},lineNumber:203},{name:`lattice_or_expr`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_and_expr`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`literal`,value:`or`},{type:`rule_reference`,name:`lattice_and_expr`}]}}]},lineNumber:205},{name:`lattice_and_expr`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_comparison`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`literal`,value:`and`},{type:`rule_reference`,name:`lattice_comparison`}]}}]},lineNumber:207},{name:`lattice_comparison`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_additive`},{type:`optional`,element:{type:`sequence`,elements:[{type:`rule_reference`,name:`comparison_op`},{type:`rule_reference`,name:`lattice_additive`}]}}]},lineNumber:209},{name:`comparison_op`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`EQUALS_EQUALS`},{type:`token_reference`,name:`NOT_EQUALS`},{type:`token_reference`,name:`GREATER`},{type:`token_reference`,name:`GREATER_EQUALS`},{type:`token_reference`,name:`LESS`},{type:`token_reference`,name:`LESS_EQUALS`}]},lineNumber:211},{name:`lattice_additive`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_multiplicative`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`}]}},{type:`rule_reference`,name:`lattice_multiplicative`}]}}]},lineNumber:214},{name:`lattice_multiplicative`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_unary`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`SLASH`}]}},{type:`rule_reference`,name:`lattice_unary`}]}}]},lineNumber:219},{name:`lattice_unary`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`MINUS`},{type:`rule_reference`,name:`lattice_unary`}]},{type:`rule_reference`,name:`lattice_primary`}]},lineNumber:221},{name:`lattice_primary`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`literal`,value:`true`},{type:`literal`,value:`false`},{type:`literal`,value:`null`},{type:`rule_reference`,name:`function_call`},{type:`rule_reference`,name:`map_literal`},{type:`sequence`,elements:[{type:`token_reference`,name:`LPAREN`},{type:`rule_reference`,name:`lattice_expression`},{type:`token_reference`,name:`RPAREN`}]}]},lineNumber:224},{name:`map_literal`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LPAREN`},{type:`rule_reference`,name:`map_entry`},{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`map_entry`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`map_entry`}]}},{type:`token_reference`,name:`RPAREN`}]},lineNumber:235},{name:`map_entry`,body:{type:`sequence`,elements:[{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STRING`}]}},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`lattice_expression`}]},lineNumber:237},{name:`function_definition`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`literal`,value:`@function`},{type:`token_reference`,name:`FUNCTION`},{type:`optional`,element:{type:`rule_reference`,name:`mixin_params`}},{type:`token_reference`,name:`RPAREN`},{type:`rule_reference`,name:`function_body`}]},{type:`sequence`,elements:[{type:`literal`,value:`@function`},{type:`token_reference`,name:`IDENT`},{type:`rule_reference`,name:`function_body`}]}]},lineNumber:261},{name:`function_body`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACE`},{type:`repetition`,element:{type:`rule_reference`,name:`function_body_item`}},{type:`token_reference`,name:`RBRACE`}]},lineNumber:264},{name:`function_body_item`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`variable_declaration`},{type:`rule_reference`,name:`return_directive`},{type:`rule_reference`,name:`lattice_control`}]},lineNumber:266},{name:`return_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@return`},{type:`rule_reference`,name:`lattice_expression`},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:268},{name:`use_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@use`},{type:`token_reference`,name:`STRING`},{type:`optional`,element:{type:`sequence`,elements:[{type:`literal`,value:`as`},{type:`token_reference`,name:`IDENT`}]}},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:281},{name:`at_rule`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`AT_KEYWORD`},{type:`rule_reference`,name:`at_prelude`},{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`SEMICOLON`},{type:`rule_reference`,name:`block`}]}}]},lineNumber:294},{name:`at_prelude`,body:{type:`repetition`,element:{type:`rule_reference`,name:`at_prelude_token`}},lineNumber:296},{name:`at_prelude_token`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`UNICODE_RANGE`},{type:`token_reference`,name:`VARIABLE`},{type:`rule_reference`,name:`function_in_prelude`},{type:`rule_reference`,name:`paren_block`},{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`DOT`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`},{type:`token_reference`,name:`GREATER`},{type:`token_reference`,name:`TILDE`},{type:`token_reference`,name:`PIPE`},{type:`token_reference`,name:`EQUALS`},{type:`token_reference`,name:`AMPERSAND`},{type:`token_reference`,name:`CDO`},{type:`token_reference`,name:`CDC`}]},lineNumber:298},{name:`function_in_prelude`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`at_prelude_tokens`},{type:`token_reference`,name:`RPAREN`}]},lineNumber:306},{name:`paren_block`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LPAREN`},{type:`rule_reference`,name:`at_prelude_tokens`},{type:`token_reference`,name:`RPAREN`}]},lineNumber:307},{name:`at_prelude_tokens`,body:{type:`repetition`,element:{type:`rule_reference`,name:`at_prelude_token`}},lineNumber:308},{name:`qualified_rule`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`selector_list`},{type:`rule_reference`,name:`block`}]},lineNumber:314},{name:`selector_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`complex_selector`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`complex_selector`}]}}]},lineNumber:320},{name:`complex_selector`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`compound_selector`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`optional`,element:{type:`rule_reference`,name:`combinator`}},{type:`rule_reference`,name:`compound_selector`}]}}]},lineNumber:322},{name:`combinator`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`GREATER`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`TILDE`}]},lineNumber:324},{name:`compound_selector`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`rule_reference`,name:`simple_selector`},{type:`repetition`,element:{type:`rule_reference`,name:`subclass_selector`}}]},{type:`sequence`,elements:[{type:`rule_reference`,name:`subclass_selector`},{type:`repetition`,element:{type:`rule_reference`,name:`subclass_selector`}}]}]},lineNumber:326},{name:`simple_selector`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`AMPERSAND`},{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`PERCENTAGE`}]},lineNumber:331},{name:`subclass_selector`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`class_selector`},{type:`rule_reference`,name:`id_selector`},{type:`rule_reference`,name:`placeholder_selector`},{type:`rule_reference`,name:`attribute_selector`},{type:`rule_reference`,name:`pseudo_class`},{type:`rule_reference`,name:`pseudo_element`}]},lineNumber:334},{name:`placeholder_selector`,body:{type:`token_reference`,name:`PLACEHOLDER`},lineNumber:338},{name:`class_selector`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`DOT`},{type:`token_reference`,name:`IDENT`}]},lineNumber:340},{name:`id_selector`,body:{type:`token_reference`,name:`HASH`},lineNumber:342},{name:`attribute_selector`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACKET`},{type:`token_reference`,name:`IDENT`},{type:`optional`,element:{type:`sequence`,elements:[{type:`rule_reference`,name:`attr_matcher`},{type:`rule_reference`,name:`attr_value`},{type:`optional`,element:{type:`token_reference`,name:`IDENT`}}]}},{type:`token_reference`,name:`RBRACKET`}]},lineNumber:344},{name:`attr_matcher`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`EQUALS`},{type:`token_reference`,name:`TILDE_EQUALS`},{type:`token_reference`,name:`PIPE_EQUALS`},{type:`token_reference`,name:`CARET_EQUALS`},{type:`token_reference`,name:`DOLLAR_EQUALS`},{type:`token_reference`,name:`STAR_EQUALS`}]},lineNumber:346},{name:`attr_value`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STRING`}]},lineNumber:349},{name:`pseudo_class`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`pseudo_class_args`},{type:`token_reference`,name:`RPAREN`}]},{type:`sequence`,elements:[{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`IDENT`}]}]},lineNumber:351},{name:`pseudo_class_args`,body:{type:`repetition`,element:{type:`rule_reference`,name:`pseudo_class_arg`}},lineNumber:354},{name:`pseudo_class_arg`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`DOT`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`AMPERSAND`},{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`pseudo_class_args`},{type:`token_reference`,name:`RPAREN`}]},{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACKET`},{type:`rule_reference`,name:`pseudo_class_args`},{type:`token_reference`,name:`RBRACKET`}]}]},lineNumber:356},{name:`pseudo_element`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`COLON_COLON`},{type:`token_reference`,name:`IDENT`}]},lineNumber:361},{name:`block`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACE`},{type:`rule_reference`,name:`block_contents`},{type:`token_reference`,name:`RBRACE`}]},lineNumber:371},{name:`block_contents`,body:{type:`repetition`,element:{type:`rule_reference`,name:`block_item`}},lineNumber:373},{name:`block_item`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`lattice_block_item`},{type:`rule_reference`,name:`at_rule`},{type:`rule_reference`,name:`declaration_or_nested`}]},lineNumber:375},{name:`lattice_block_item`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`variable_declaration`},{type:`rule_reference`,name:`include_directive`},{type:`rule_reference`,name:`lattice_control`},{type:`rule_reference`,name:`content_directive`},{type:`rule_reference`,name:`extend_directive`},{type:`rule_reference`,name:`at_root_directive`}]},lineNumber:381},{name:`content_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@content`},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:391},{name:`extend_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@extend`},{type:`rule_reference`,name:`selector_list`},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:399},{name:`at_root_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@at-root`},{type:`group`,element:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`rule_reference`,name:`selector_list`},{type:`rule_reference`,name:`block`}]},{type:`rule_reference`,name:`block`}]}}]},lineNumber:404},{name:`declaration_or_nested`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`declaration`},{type:`rule_reference`,name:`qualified_rule`}]},lineNumber:406},{name:`declaration`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`rule_reference`,name:`property`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`value_list`},{type:`optional`,element:{type:`rule_reference`,name:`priority`}},{type:`token_reference`,name:`SEMICOLON`}]},{type:`sequence`,elements:[{type:`rule_reference`,name:`property`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`block`}]}]},lineNumber:415},{name:`property`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`CUSTOM_PROPERTY`}]},lineNumber:418},{name:`priority`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`BANG`},{type:`literal`,value:`important`}]},lineNumber:420},{name:`value_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`value`},{type:`repetition`,element:{type:`rule_reference`,name:`value`}}]},lineNumber:431},{name:`value`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`UNICODE_RANGE`},{type:`rule_reference`,name:`function_call`},{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`},{type:`rule_reference`,name:`map_literal`}]},lineNumber:433},{name:`function_call`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`function_args`},{type:`token_reference`,name:`RPAREN`}]},{type:`token_reference`,name:`URL_TOKEN`}]},lineNumber:439},{name:`function_args`,body:{type:`repetition`,element:{type:`rule_reference`,name:`function_arg`}},lineNumber:442},{name:`function_arg`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`VARIABLE`},{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`function_args`},{type:`token_reference`,name:`RPAREN`}]}]},lineNumber:444}]};function Xd(e){return new Nd(Jd(e),Yd)}function Zd(e){return Xd(e).parse()}var Qd=class extends Error{latticeMessage;line;column;constructor(e,t=0,n=0){let r=t?` at line ${t}, column ${n}`:``;super(`${e}${r}`),this.latticeMessage=e,this.line=t,this.column=n,Object.setPrototypeOf(this,new.target.prototype),this.name=new.target.name}};function $d(e,t){let n=e.length+1,r=t.length+1,i=Array.from({length:n},()=>Array(r).fill(0));for(let e=0;e<n;e+=1)i[e][0]=e;for(let e=0;e<r;e+=1)i[0][e]=e;for(let a=1;a<n;a+=1)for(let n=1;n<r;n+=1){let r=e[a-1]===t[n-1]?0:1;i[a][n]=Math.min(i[a-1][n]+1,i[a][n-1]+1,i[a-1][n-1]+r)}return i[n-1][r-1]}function ef(e,t){return[...t].map(t=>({candidate:t,distance:$d(e,t)})).filter(({candidate:t,distance:n})=>t.includes(e)||e.includes(t)||n<=3).sort((e,t)=>e.distance-t.distance||e.candidate.localeCompare(t.candidate)).slice(0,3).map(({candidate:e})=>e)}var tf=class extends Qd{name;constructor(e,t=0,n=0){super(`Undefined variable '${e}'`,t,n),this.name=e}},nf=class extends Qd{name;suggestions;constructor(e,t=0,n=0,r=[]){let i=ef(e,r),a=[`Undefined mixin '${e}'.`];r.length===0?a.push(`No mixins are currently defined in scope.`):i.length>0?a.push(`Did you mean ${i.map(e=>`'${e}'`).join(` or `)}?`):a.push(`Defined mixins in scope: ${r.sort().join(`, `)}.`),a.push("If this is a zero-argument mixin, both `@mixin card() { ... }` and `@mixin card { ... }` are valid."),super(a.join(` `),t,n),this.name=e,this.suggestions=i}},rf=class extends Qd{name;expected;got;constructor(e,t,n,r,i=0,a=0){super(`${e} '${t}' expects ${n} args, got ${r}`,i,a),this.name=t,this.expected=n,this.got=r}},af=class extends Qd{chain;constructor(e,t,n=0,r=0){let i=t.join(` → `);super(`Circular ${e}: ${i}`,n,r),this.chain=t}},X=class extends Qd{op;leftType;rightType;constructor(e,t,n,r=0,i=0){super(`Cannot ${e} '${t}' and '${n}'`,r,i),this.op=e,this.leftType=t,this.rightType=n}},of=class extends Qd{name;constructor(e,t=0,n=0){super(`Function '${e}' has no @return`,t,n),this.name=e}},sf=class extends Qd{maxIterations;constructor(e=1e3,t=0,n=0){super(`@while loop exceeded maximum iteration count (${e})`,t,n),this.maxIterations=e}},cf=class extends Qd{constructor(e,t=0,n=0){super(e,t,n)}},lf=class extends Qd{constructor(e=0,t=0){super(`Division by zero`,e,t)}},uf=class e{bindings=new Map;parent;constructor(e=null){this.parent=e}get(e){if(this.bindings.has(e))return this.bindings.get(e);if(this.parent!==null)return this.parent.get(e)}set(e,t){this.bindings.set(e,t)}has(e){return this.bindings.has(e)?!0:this.parent===null?!1:this.parent.has(e)}hasLocal(e){return this.bindings.has(e)}setGlobal(e,t){let n=this;for(;n.parent!==null;)n=n.parent;n.set(e,t)}child(){return new e(this)}get depth(){return this.parent===null?0:1+this.parent.depth}toString(){let e=Array.from(this.bindings.keys());return`ScopeChain(depth=${this.depth}, bindings=[${e.join(`, `)}])`}};function df(e){return e>=`A`&&e<=`Z`||e>=`a`&&e<=`z`}function ff(e){let t=0;e[t]===`-`&&t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;let r=t-n,i=0;if(e[t]===`.`){t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;i=t-n}if(r===0&&i===0)return null;if(e[t]===`e`||e[t]===`E`){let n=t;t++,(e[t]===`+`||e[t]===`-`)&&t++;let r=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;t===r&&(t=n)}let a=e.slice(t);if(a.length===0)return null;for(let e of a)if(!df(e))return null;return{numberPart:e.slice(0,t),unit:a}}var pf=class{value;kind=`number`;constructor(e){this.value=e}toString(){return this.value===Math.trunc(this.value)&&isFinite(this.value)?String(Math.trunc(this.value)):String(this.value)}},mf=class{value;unit;kind=`dimension`;constructor(e,t){this.value=e,this.unit=t}toString(){return this.value===Math.trunc(this.value)&&isFinite(this.value)?`${Math.trunc(this.value)}${this.unit}`:`${this.value}${this.unit}`}},hf=class{value;kind=`percentage`;constructor(e){this.value=e}toString(){return this.value===Math.trunc(this.value)&&isFinite(this.value)?`${Math.trunc(this.value)}%`:`${this.value}%`}},gf=class{value;kind=`string`;constructor(e){this.value=e}toString(){return`"${this.value}"`}},_f=class{value;kind=`ident`;constructor(e){this.value=e}toString(){return this.value}},vf=class e{value;kind=`color`;constructor(e){this.value=e}toRgb(){let e=this.value.replace(/^#/,``);return e.length===3?[parseInt(e[0]+e[0],16),parseInt(e[1]+e[1],16),parseInt(e[2]+e[2],16),1]:e.length===6?[parseInt(e.slice(0,2),16),parseInt(e.slice(2,4),16),parseInt(e.slice(4,6),16),1]:e.length===8?[parseInt(e.slice(0,2),16),parseInt(e.slice(2,4),16),parseInt(e.slice(4,6),16),parseInt(e.slice(6,8),16)/255]:[0,0,0,1]}toHsl(){let[e,t,n,r]=this.toRgb(),i=e/255,a=t/255,o=n/255,s=Math.max(i,a,o),c=Math.min(i,a,o),l=(s+c)/2;if(s===c)return[0,0,l*100,r];let u=s-c,d=l>.5?u/(2-s-c):u/(s+c),f;return f=s===i?(a-o)/u+(a<o?6:0):s===a?(o-i)/u+2:(i-a)/u+4,f*=60,[f,d*100,l*100,r]}static fromRgb(t,n,r,i=1){return t=Math.max(0,Math.min(255,Math.round(t))),n=Math.max(0,Math.min(255,Math.round(n))),r=Math.max(0,Math.min(255,Math.round(r))),i=Math.max(0,Math.min(1,i)),i>=1?new e(`#${t.toString(16).padStart(2,`0`)}${n.toString(16).padStart(2,`0`)}${r.toString(16).padStart(2,`0`)}`):new e(`rgba(${t}, ${n}, ${r}, ${i})`)}static fromHsl(t,n,r,i=1){if(t=(t%360+360)%360,n=Math.max(0,Math.min(100,n))/100,r=Math.max(0,Math.min(100,r))/100,n===0){let t=Math.round(r*255);return e.fromRgb(t,t,t,i)}let a=r<.5?r*(1+n):r+n-r*n,o=2*r-a;function s(e,t,n){return n<0&&(n+=1),n>1&&--n,n<1/6?e+(t-e)*6*n:n<1/2?t:n<2/3?e+(t-e)*(2/3-n)*6:e}let c=t/360,l=Math.round(s(o,a,c+1/3)*255),u=Math.round(s(o,a,c)*255),d=Math.round(s(o,a,c-1/3)*255);return e.fromRgb(l,u,d,i)}toString(){return this.value}},yf=class{value;kind=`bool`;constructor(e){this.value=e}toString(){return this.value?`true`:`false`}},bf=class{kind=`null`;toString(){return``}},xf=class{items;kind=`list`;constructor(e){this.items=e}toString(){return this.items.map(e=>e.toString()).join(`, `)}},Sf=class{items;kind=`map`;constructor(e){this.items=e}get(e){for(let[t,n]of this.items)if(t===e)return n}keys(){return this.items.map(([e])=>e)}values(){return this.items.map(([,e])=>e)}hasKey(e){return this.items.some(([t])=>t===e)}toString(){return`(${this.items.map(([e,t])=>`${e}: ${t}`).join(`, `)})`}};function Cf(e){return e.kind===`bool`?e.value:!(e.kind===`null`||e.kind===`number`&&e.value===0)}function wf(e){let{type:t,value:n}=e;if(t===`NUMBER`)return new pf(parseFloat(n));if(t===`DIMENSION`){let e=ff(n);if(e)return new mf(parseFloat(e.numberPart),e.unit);let t=0;for(n[t]===`-`&&t++;t<n.length&&(n[t]===`.`||n[t]>=`0`&&n[t]<=`9`);)t++;return new mf(parseFloat(n.slice(0,t)),n.slice(t))}return t===`PERCENTAGE`?new hf(parseFloat(n.replace(`%`,``))):t===`STRING`?new gf(n):t===`HASH`?new vf(n):t===`IDENT`?n===`true`?new yf(!0):n===`false`?new yf(!1):n===`null`?new bf:new _f(n):new _f(String(n))}function Tf(e){if(!(`ruleName`in e))return wf(e);let t=e;for(let e of t.children)if(`ruleName`in e){let t=Tf(e);if(t.kind!==`null`)return t}else return wf(e);return new bf}function Ef(e){return e.toString()}function Df(e,t){if(e.kind===`number`&&t.kind===`number`)return new pf(e.value+t.value);if(e.kind===`dimension`&&t.kind===`dimension`){if(e.unit===t.unit)return new mf(e.value+t.value,e.unit);throw new X(`add`,e.toString(),t.toString())}if(e.kind===`percentage`&&t.kind===`percentage`)return new hf(e.value+t.value);if(e.kind===`string`&&t.kind===`string`)return new gf(e.value+t.value);throw new X(`add`,e.toString(),t.toString())}function Of(e,t){if(e.kind===`number`&&t.kind===`number`)return new pf(e.value-t.value);if(e.kind===`dimension`&&t.kind===`dimension`){if(e.unit===t.unit)return new mf(e.value-t.value,e.unit);throw new X(`subtract`,e.toString(),t.toString())}if(e.kind===`percentage`&&t.kind===`percentage`)return new hf(e.value-t.value);throw new X(`subtract`,e.toString(),t.toString())}function kf(e,t){if(e.kind===`number`&&t.kind===`number`)return new pf(e.value*t.value);if(e.kind===`number`&&t.kind===`dimension`)return new mf(e.value*t.value,t.unit);if(e.kind===`dimension`&&t.kind===`number`)return new mf(e.value*t.value,e.unit);if(e.kind===`number`&&t.kind===`percentage`||e.kind===`percentage`&&t.kind===`number`)return new hf(e.value*t.value);throw new X(`multiply`,e.toString(),t.toString())}function Af(e,t){let n=()=>{if(t.kind===`number`||t.kind===`dimension`||t.kind===`percentage`){if(t.value===0)throw new lf;return t.value}throw new X(`divide`,e.toString(),t.toString())};if(e.kind===`number`&&t.kind===`number`){if(t.value===0)throw new lf;return new pf(e.value/t.value)}if(e.kind===`dimension`&&t.kind===`number`){if(t.value===0)throw new lf;return new mf(e.value/t.value,e.unit)}if(e.kind===`dimension`&&t.kind===`dimension`&&e.unit===t.unit){if(t.value===0)throw new lf;return new pf(e.value/t.value)}if(e.kind===`percentage`&&t.kind===`number`){if(t.value===0)throw new lf;return new hf(e.value/t.value)}throw n(),e.kind===`number`||e.kind===`dimension`||e.kind,new X(`divide`,e.toString(),t.toString())}function jf(e){if(e.kind===`number`)return new pf(-e.value);if(e.kind===`dimension`)return new mf(-e.value,e.unit);if(e.kind===`percentage`)return new hf(-e.value);throw new X(`negate`,e.toString(),``)}function Mf(e,t,n){if((e=>e.kind===`number`||e.kind===`dimension`||e.kind===`percentage`)(e)&&e.kind===t.kind){let r=e.value,i=t.value;if(e.kind===`dimension`&&t.kind===`dimension`&&e.unit!==t.unit&&n!==`EQUALS_EQUALS`&&n!==`NOT_EQUALS`)return new yf(!1);switch(n){case`EQUALS_EQUALS`:return e.kind===`dimension`&&t.kind===`dimension`?new yf(r===i&&e.unit===t.unit):new yf(r===i);case`NOT_EQUALS`:return e.kind===`dimension`&&t.kind===`dimension`?new yf(r!==i||e.unit!==t.unit):new yf(r!==i);case`GREATER`:return new yf(r>i);case`GREATER_EQUALS`:return new yf(r>=i);case`LESS`:return new yf(r<i);case`LESS_EQUALS`:return new yf(r<=i)}}let r=e.toString(),i=t.toString();return n===`EQUALS_EQUALS`?new yf(r===i):n===`NOT_EQUALS`?new yf(r!==i):new yf(!1)}function Nf(e){switch(e.kind){case`number`:case`dimension`:case`percentage`:return`number`;case`string`:case`ident`:return`string`;case`color`:return`color`;case`bool`:return`bool`;case`null`:return`null`;case`list`:return`list`;case`map`:return`map`;default:return`unknown`}}function Pf(e){if(e.kind===`number`||e.kind===`dimension`||e.kind===`percentage`)return e.value;throw new X(`use`,`Expected a number, got ${Nf(e)}`,``)}function Ff(e){if(e.kind!==`color`)throw new X(`use`,`Expected a color, got ${Nf(e)}`,``);return e}function If(e){let t=Pf(e);if(t<0||t>100)throw new cf(`Amount must be between 0% and 100%`);return t}function Lf(e){if(e.kind!==`map`)throw new X(`use`,`Expected a map, got ${Nf(e)}`,``);return e}var Rf=e=>{if(e.length<2)throw new X(`call`,`map-get requires 2 arguments`,``);let t=Lf(e[0]),n=e[1].toString().replace(/^"|"$/g,``);return t.get(n)??new bf},zf=e=>{if(!e.length)throw new X(`call`,`map-keys requires 1 argument`,``);return new xf(Lf(e[0]).keys().map(e=>new _f(e)))},Bf=e=>{if(!e.length)throw new X(`call`,`map-values requires 1 argument`,``);return new xf(Lf(e[0]).values())},Vf=e=>{if(e.length<2)throw new X(`call`,`map-has-key requires 2 arguments`,``);let t=Lf(e[0]),n=e[1].toString().replace(/^"|"$/g,``);return new yf(t.hasKey(n))},Hf=e=>{if(e.length<2)throw new X(`call`,`map-merge requires 2 arguments`,``);let t=Lf(e[0]),n=Lf(e[1]),r=new Map;for(let[e,n]of t.items)r.set(e,n);for(let[e,t]of n.items)r.set(e,t);return new Sf(Array.from(r.entries()))},Uf=e=>{if(!e.length)throw new X(`call`,`map-remove requires at least 1 argument`,``);let t=Lf(e[0]),n=new Set(e.slice(1).map(e=>e.toString().replace(/^"|"$/g,``)));return new Sf(t.items.filter(([e])=>!n.has(e)))},Wf=e=>{let t=Ff(e[0]),n=If(e[1]),[r,i,a,o]=t.toHsl();return vf.fromHsl(r,i,Math.min(100,a+n),o)},Gf=e=>{let t=Ff(e[0]),n=If(e[1]),[r,i,a,o]=t.toHsl();return vf.fromHsl(r,i,Math.max(0,a-n),o)},Kf=e=>{let t=Ff(e[0]),n=If(e[1]),[r,i,a,o]=t.toHsl();return vf.fromHsl(r,Math.min(100,i+n),a,o)},qf=e=>{let t=Ff(e[0]),n=If(e[1]),[r,i,a,o]=t.toHsl();return vf.fromHsl(r,Math.max(0,i-n),a,o)},Jf=e=>{let t=Ff(e[0]),n=Pf(e[1]),[r,i,a,o]=t.toHsl();return vf.fromHsl((r+n)%360,i,a,o)},Yf=e=>{let[t,n,r,i]=Ff(e[0]).toHsl();return vf.fromHsl((t+180)%360,n,r,i)},Xf=e=>{let t=Ff(e[0]),n=Ff(e[1]),r=(e.length>=3?Pf(e[2]):50)/100,[i,a,o,s]=t.toRgb(),[c,l,u,d]=n.toRgb();return vf.fromRgb(Math.round(i*r+c*(1-r)),Math.round(a*r+l*(1-r)),Math.round(o*r+u*(1-r)),s*r+d*(1-r))},Zf=e=>{if(e.length===2&&e[0].kind===`color`){let t=e[0],n=Pf(e[1]),[r,i,a]=t.toRgb();return vf.fromRgb(r,i,a,n)}return e.length===4?vf.fromRgb(Math.round(Pf(e[0])),Math.round(Pf(e[1])),Math.round(Pf(e[2])),Pf(e[3])):new bf},Qf=e=>{let[t]=Ff(e[0]).toRgb();return new pf(t)},$f=e=>{let[,t]=Ff(e[0]).toRgb();return new pf(t)},ep=e=>{let[,,t]=Ff(e[0]).toRgb();return new pf(t)},tp=e=>{let[t]=Ff(e[0]).toHsl();return new mf(Math.round(t),`deg`)},np=e=>{let[,t]=Ff(e[0]).toHsl();return new hf(Math.round(t))},rp=e=>{let[,,t]=Ff(e[0]).toHsl();return new hf(Math.round(t))},ip=e=>{if(e.length<2)throw new X(`call`,`nth requires 2 arguments`,``);let t=e[0],n=Math.trunc(Pf(e[1]));if(n<1)throw new cf(`List index must be 1 or greater`);if(t.kind===`list`){if(n>t.items.length)throw new cf(`Index ${n} out of bounds for list of length ${t.items.length}`);return t.items[n-1]}if(n===1)return t;throw new cf(`Index ${n} out of bounds for list of length 1`)},ap=e=>{if(!e.length)throw new X(`call`,`length requires 1 argument`,``);let t=e[0];return t.kind===`list`||t.kind===`map`?new pf(t.items.length):new pf(1)},op=e=>{if(e.length<2)throw new X(`call`,`join requires at least 2 arguments`,``);let t=e[0].kind===`list`?e[0].items:[e[0]],n=e[1].kind===`list`?e[1].items:[e[1]];return new xf([...t,...n])},sp=e=>{if(e.length<2)throw new X(`call`,`append requires at least 2 arguments`,``);let t=e[0].kind===`list`?[...e[0].items]:[e[0]];return t.push(e[1]),new xf(t)},cp=e=>{if(e.length<2)throw new X(`call`,`index requires 2 arguments`,``);let t=e[0].kind===`list`?e[0].items:[e[0]],n=e[1].toString();for(let e=0;e<t.length;e++)if(t[e].toString()===n)return new pf(e+1);return new bf},lp=e=>{if(!e.length)throw new X(`call`,`type-of requires 1 argument`,``);return new gf(Nf(e[0]))},up=e=>{if(!e.length)throw new X(`call`,`unit requires 1 argument`,``);let t=e[0];if(t.kind===`dimension`)return new gf(t.unit);if(t.kind===`percentage`)return new gf(`%`);if(t.kind===`number`)return new gf(``);throw new X(`use`,`Expected a number, got ${Nf(t)}`,``)},dp=e=>{if(!e.length)throw new X(`call`,`unitless requires 1 argument`,``);return new yf(e[0].kind===`number`)},fp=e=>{if(e.length<2)throw new X(`call`,`comparable requires 2 arguments`,``);let t=e[0],n=e[1];if(t.kind===n.kind)return t.kind===`dimension`&&n.kind===`dimension`?new yf(t.unit===n.unit):new yf(!0);let r=e=>e===`number`||e===`dimension`||e===`percentage`;return r(t.kind)&&r(n.kind)&&(t.kind===`number`||n.kind===`number`)?new yf(!0):new yf(!1)},pp=e=>{if(e.length<2)throw new X(`call`,`math.div requires 2 arguments`,``);let t=e[0],n=e[1],r=Pf(n);if(r===0)throw new lf;let i=Pf(t);return t.kind===`dimension`&&n.kind===`number`?new mf(i/r,t.unit):t.kind===`dimension`&&n.kind===`dimension`&&t.unit===n.unit?new pf(i/r):t.kind===`percentage`&&n.kind===`number`?new hf(i/r):new pf(i/r)};function mp(e){return t=>{if(!t.length)throw new X(`call`,`math function requires 1 argument`,``);let n=t[0],r=e(Pf(n));return n.kind===`dimension`?new mf(r,n.unit):n.kind===`percentage`?new hf(r):new pf(r)}}var hp=mp(Math.floor),gp=mp(Math.ceil),_p=mp(Math.round),vp=mp(Math.abs),yp=new Map([[`map-get`,Rf],[`map-keys`,zf],[`map-values`,Bf],[`map-has-key`,Vf],[`map-merge`,Hf],[`map-remove`,Uf],[`lighten`,Wf],[`darken`,Gf],[`saturate`,Kf],[`desaturate`,qf],[`adjust-hue`,Jf],[`complement`,Yf],[`mix`,Xf],[`rgba`,Zf],[`red`,Qf],[`green`,$f],[`blue`,ep],[`hue`,tp],[`saturation`,np],[`lightness`,rp],[`nth`,ip],[`length`,ap],[`join`,op],[`append`,sp],[`index`,cp],[`type-of`,lp],[`unit`,up],[`unitless`,dp],[`comparable`,fp],[`math.div`,pp],[`math.floor`,hp],[`math.ceil`,gp],[`math.round`,_p],[`math.abs`,vp],[`math.min`,e=>{if(!e.length)throw new X(`call`,`math.min requires at least 1 argument`,``);let t=e[0],n=Pf(t);for(let r=1;r<e.length;r++){let i=Pf(e[r]);i<n&&(t=e[r],n=i)}return t}],[`math.max`,e=>{if(!e.length)throw new X(`call`,`math.max requires at least 1 argument`,``);let t=e[0],n=Pf(t);for(let r=1;r<e.length;r++){let i=Pf(e[r]);i>n&&(t=e[r],n=i)}return t}]]);function bp(e){return`ruleName`in e}function xp(e){return e.type}var Sp=class{scope;constructor(e){this.scope=e}evaluate(e){if(!bp(e))return wf(e);switch(e.ruleName){case`lattice_expression`:return this._evalLatticeExpression(e);case`lattice_or_expr`:return this._evalOrExpr(e);case`lattice_and_expr`:return this._evalAndExpr(e);case`lattice_comparison`:return this._evalComparison(e);case`lattice_additive`:return this._evalAdditive(e);case`lattice_multiplicative`:return this._evalMultiplicative(e);case`lattice_unary`:return this._evalUnary(e);case`lattice_primary`:return this._evalPrimary(e);case`comparison_op`:return wf(e.children[0]);case`value_list`:return this._evalValueList(e)}let t=e.children;if(t.length===1)return this.evaluate(t[0]);for(let e of t)if(bp(e)||e.type)return this.evaluate(e);return new bf}_evalLatticeExpression(e){return this.evaluate(e.children[0])}_evalOrExpr(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!bp(e)&&e.value===`or`){r++;continue}if(Cf(n))return n;n=this.evaluate(e),r++}return n}_evalAndExpr(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!bp(e)&&e.value===`and`){r++;continue}if(!Cf(n))return n;n=this.evaluate(e),r++}return n}_evalComparison(e){let t=e.children,n=this.evaluate(t[0]);if(t.length===1)return n;let r=null,i=null;for(let e=1;e<t.length;e++){let n=t[e];if(bp(n)&&n.ruleName===`comparison_op`)r=n;else if(r!==null){i=n;break}}if(r===null||i===null)return n;let a=this.evaluate(i),o=r.children[0];return Mf(n,a,xp(o))}_evalValueList(e){let t=e.children;return t.length<=1?t.length===0?new bf:this.evaluate(t[0]):t.some(e=>!bp(e)&&e.value!==void 0&&[`+`,`-`,`*`].includes(e.value))?this._evalAdditive(e):this.evaluate(t[0])}_evalAdditive(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!bp(e)){let i=e.value;if((i===`+`||i===`-`)&&(r++,r<t.length)){let e=this.evaluate(t[r]);n=i===`+`?Df(n,e):Of(n,e)}}r++}return n}_evalMultiplicative(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!bp(e)){let i=e.value;if((i===`*`||i===`/`)&&(r++,r<t.length)){let e=this.evaluate(t[r]);n=i===`*`?kf(n,e):Af(n,e)}}r++}return n}_evalUnary(e){let t=e.children;return t.length>=2&&!bp(t[0])&&t[0].value===`-`?jf(this.evaluate(t[1])):this.evaluate(t[0])}_evalPrimary(e){let t=e.children;for(let e of t){if(!bp(e)){let t=e,n=xp(t);if(n===`LPAREN`||n===`RPAREN`)continue;if(n===`VARIABLE`){let e=this.scope.get(t.value);return e===void 0?new _f(t.value):typeof e==`object`&&e&&`kind`in e?e:typeof e==`object`&&e&&`ruleName`in e?Tf(e):typeof e==`object`&&e&&`type`in e?wf(e):new bf}return wf(t)}return this.evaluate(e)}return new bf}};function Z(e){return`ruleName`in e}function Q(e){return e.type}function Cp(e){if(!Z(e))return e.value}var wp=new Set(`rgb.rgba.hsl.hsla.hwb.lab.lch.oklch.oklab.color.color-mix.calc.min.max.clamp.abs.sign.round.mod.rem.sin.cos.tan.asin.acos.atan.atan2.pow.sqrt.hypot.log.exp.var.env.url.format.local.linear-gradient.radial-gradient.conic-gradient.repeating-linear-gradient.repeating-radial-gradient.repeating-conic-gradient.counter.counters.attr.element.translate.translateX.translateY.translateZ.rotate.rotateX.rotateY.rotateZ.scale.scaleX.scaleY.scaleZ.skew.skewX.skewY.matrix.matrix3d.perspective.cubic-bezier.steps.path.polygon.circle.ellipse.inset.image-set.cross-fade.fit-content.minmax.repeat.blur.brightness.contrast.drop-shadow.grayscale.hue-rotate.invert.opacity.saturate.sepia`.split(`.`));function Tp(e){let t=e.replace(/\($/,``);return wp.has(t)}var Ep=class{value;constructor(e){this.value=e}},Dp=class{ruleName;children;constructor(e,t){this.ruleName=e,this.children=t}};function Op(e){return e>=`A`&&e<=`Z`||e>=`a`&&e<=`z`}function kp(e){let t=0;e[t]===`-`&&t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;let r=t-n,i=0;if(e[t]===`.`){t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;i=t-n}return r===0&&i===0?-1:t}function Ap(e){if(e.length===0)return!1;for(let t of e)if(!Op(t))return!1;return!0}function jp(e,t){let n=Z(t)?0:t.line??0,r=Z(t)?0:t.column??0,i=`IDENT`;if(e.startsWith(`#`))i=`HASH`;else if(e.startsWith(`"`)||e.startsWith(`'`))i=`STRING`;else{let t=kp(e);t===e.length-1&&e[t]===`%`?i=`PERCENTAGE`:t>0&&t<e.length&&Ap(e.slice(t))?i=`DIMENSION`:t===e.length&&(i=`NUMBER`)}return{type:i,value:e,line:n,column:r}}function Mp(e,t){return new Dp(`value`,[jp(e,t)])}function Np(e){if(!Z(e))return{...e};let t=e;return new Dp(t.ruleName,t.children.map(e=>Np(e)))}function $(e){return e.children}function Pp(e,t){e.children=t}var Fp=class{variables=new uf;mixins=new Map;functions=new Map;mixinStack=[];functionStack=[];maxWhileIterations;extendMap=new Map;atRootRules=[];contentBlockStack=[];contentScopeStack=[];constructor(e=1e3){this.maxWhileIterations=e}transform(e){this._collectSymbols(e);let t=this._expandNode(e,this.variables),n=this._cleanup(t);return this.extendMap.size>0&&this._applyExtends(n),this.atRootRules.length>0&&this._spliceAtRootRules(n),n}_collectSymbols(e){if(!Z(e))return;let t=[];for(let n of $(e)){if(!Z(n)){t.push(n);continue}let e=n;if(e.ruleName===`rule`){let r=$(e);if(r.length===0){t.push(n);continue}let i=r[0];if(!Z(i)){t.push(n);continue}let a=i;if(a.ruleName===`lattice_rule`){let e=$(a);if(e.length===0){t.push(n);continue}let r=e[0];if(!Z(r)){t.push(n);continue}let i=r.ruleName;if(i===`variable_declaration`){this._collectVariable(r);continue}else if(i===`mixin_definition`){this._collectMixin(r);continue}else if(i===`function_definition`){this._collectFunction(r);continue}else if(i===`use_directive`)continue}t.push(n)}else t.push(n)}Pp(e,t)}_collectVariable(e){let t,n,r=!1,i=!1;for(let a of $(e))if(Z(a)){let e=a;if(e.ruleName===`value_list`)n=e;else if(e.ruleName===`variable_flag`){for(let t of $(e))if(!Z(t)){let e=Q(t);e===`BANG_DEFAULT`?r=!0:e===`BANG_GLOBAL`&&(i=!0)}}}else{let e=Q(a);e===`VARIABLE`?t=a.value:e===`BANG_DEFAULT`?r=!0:e===`BANG_GLOBAL`&&(i=!0)}if(t&&n)if(r&&i){let e=this.variables;for(;e.parent!==null;)e=e.parent;e.get(t)===void 0&&this.variables.setGlobal(t,n)}else r?this.variables.get(t)===void 0&&this.variables.set(t,n):i?this.variables.setGlobal(t,n):this.variables.set(t,n)}_collectMixin(e){let t,n=[],r=new Map,i;for(let a of $(e))if(Z(a)){let e=a;if(e.ruleName===`mixin_params`){let t=this._extractParams(e);n=t.params,r=t.defaults}else e.ruleName===`block`&&(i=e)}else{let e=a,n=Q(e);n===`FUNCTION`?t=e.value.replace(/\($/,``):n===`IDENT`&&(t=e.value)}t&&i&&this.mixins.set(t,{name:t,params:n,defaults:r,body:i})}_collectFunction(e){let t,n=[],r=new Map,i;for(let a of $(e))if(!Z(a))Q(a)===`FUNCTION`&&(t=a.value.replace(/\($/,``));else{let e=a;if(e.ruleName===`mixin_params`){let t=this._extractParams(e);n=t.params,r=t.defaults}else e.ruleName===`function_body`&&(i=e)}t&&i&&this.functions.set(t,{name:t,params:n,defaults:r,body:i})}_extractParams(e){let t=[],n=new Map;for(let r of $(e)){if(!Z(r))continue;let e=r;if(e.ruleName===`mixin_param`){let r,i;for(let t of $(e))Z(t)?(t.ruleName===`value_list`||t.ruleName===`mixin_value_list`)&&(i=t):Q(t)===`VARIABLE`&&(r=t.value);r&&(t.push(r),i!==void 0&&n.set(r,i))}}return{params:t,defaults:n}}_expandNode(e,t){if(!Z(e)){let n=e;return Q(n)===`VARIABLE`?this._substituteVariable(n,t):n}let n=e;switch(n.ruleName){case`stylesheet`:return this._expandStylesheet(n,t);case`rule`:return this._expandTopLevelRule(n,t);case`lattice_rule`:return this._expandTopLevelLatticeRule(n,t);case`lattice_control`:return this._expandControl(n,t);case`block`:return this._expandBlock(n,t);case`block_contents`:return this._expandBlockContents(n,t);case`block_item`:return this._expandBlockItem(n,t);case`value_list`:return this._expandValueList(n,t);case`value`:return this._expandValue(n,t);case`function_call`:return this._expandFunctionCall(n,t);case`function_arg`:return this._expandChildren(n,t);case`function_args`:return this._expandChildren(n,t);case`compound_selector`:case`simple_selector`:case`class_selector`:return this._expandSelectorWithVars(n,t);default:return this._expandChildren(n,t)}}_expandTopLevelRule(e,t){let n=$(e);if(n.length===0)return e;let r=n[0];if(!Z(r))return this._expandChildren(e,t);let i=r;if(i.ruleName===`lattice_rule`){let n=this._expandTopLevelLatticeRule(i,t);return n===null?null:Array.isArray(n)?n:(Pp(e,[n]),e)}return this._expandChildren(e,t)}_expandTopLevelLatticeRule(e,t){let n=$(e);if(n.length===0)return null;let r=n[0];if(!Z(r))return null;let i=r,a=i.ruleName;return a===`lattice_control`?this._expandControl(i,t):a===`variable_declaration`||a===`mixin_definition`||a===`function_definition`||a===`use_directive`?null:this._expandChildren(e,t)}_expandStylesheet(e,t){let n=[];for(let r of $(e)){let e=this._expandNode(r,t);e===null||(Array.isArray(e)?n.push(...e):n.push(e))}return Pp(e,n),e}_expandChildren(e,t){let n=[];for(let r of $(e)){let e=this._expandNode(r,t);e!==null&&(Array.isArray(e)?n.push(...e):n.push(e))}return Pp(e,n),e}_substituteVariable(e,t){let n=e.value,r=t.get(n);if(r===void 0)throw new tf(n,e.line??0,e.column??0);if(typeof r==`object`&&r&&`ruleName`in r){let n=Np(r),i=this._expandNode(n,t);return i===null?jp(``,e):Array.isArray(i)?i[0]:i}return typeof r==`object`&&r&&`kind`in r?jp(Ef(r),e):e}_expandBlock(e,t){let n=t.child();return this._expandChildren(e,n)}_expandBlockContents(e,t){let n=[];for(let r of $(e)){let e=this._expandBlockItemInner(r,t);e===null||(Array.isArray(e)?n.push(...e):n.push(e))}return Pp(e,n),e}_expandBlockItemInner(e,t){if(!Z(e))return e;let n=e;if(n.ruleName===`block_item`){let e=$(n);if(e.length>0&&Z(e[0])){let r=e[0];if(r.ruleName===`lattice_block_item`){let e=this._expandLatticeBlockItem(r,t);return e===null?null:Array.isArray(e)?e:(Pp(n,[r]),Pp(r,[e]),n)}if(r.ruleName===`declaration_or_nested`){let e=$(r);if(e.length>0&&Z(e[0])&&e[0].ruleName===`property_nesting`){let n=this._expandPropertyNesting(e[0],t);return n.length>0?n:null}}}return this._expandChildren(n,t)}return this._expandChildren(n,t)}_expandBlockItem(e,t){let n=$(e);if(n.length===0)return e;let r=n[0];if(!Z(r))return this._expandChildren(e,t);let i=r;if(i.ruleName===`lattice_block_item`){let n=this._expandLatticeBlockItem(i,t);return n===null?null:Array.isArray(n)?n:(Pp(e,[n]),e)}return this._expandChildren(e,t)}_expandLatticeBlockItem(e,t){let n=$(e);if(n.length===0)return e;let r=n[0];if(!Z(r))return e;let i=r,a=i.ruleName;return a===`variable_declaration`?(this._expandVariableDeclaration(i,t),null):a===`include_directive`?this._expandInclude(i,t):a===`lattice_control`?this._expandControl(i,t):a===`content_directive`?this._expandContent(t):a===`at_root_directive`?this._expandAtRoot(i,t):a===`extend_directive`?(this._collectExtend(i),null):this._expandChildren(e,t)}_expandVariableDeclaration(e,t){let n,r,i=!1,a=!1;for(let t of $(e))if(Z(t)){let e=t;if(e.ruleName===`value_list`)r=e;else if(e.ruleName===`variable_flag`){for(let t of $(e))if(!Z(t)){let e=Q(t);e===`BANG_DEFAULT`?i=!0:e===`BANG_GLOBAL`&&(a=!0)}}}else{let e=Q(t);e===`VARIABLE`?n=t.value:e===`BANG_DEFAULT`?i=!0:e===`BANG_GLOBAL`&&(a=!0)}if(n&&r){let e=this._expandNode(Np(r),t),o=e??r;try{let n=new Sp(t).evaluate(Np(e??r));n!=null&&(o=n)}catch{}if(i&&a){let e=t;for(;e.parent!==null;)e=e.parent;e.get(n)===void 0&&t.setGlobal(n,o)}else i?t.get(n)===void 0&&t.set(n,o):a?t.setGlobal(n,o):t.set(n,o)}}_expandValueList(e,t){let n=[];for(let r of $(e)){let e=this._expandNode(r,t);if(e!==null)if(Array.isArray(e))n.push(...e);else{let t=e;Z(t)&&t.ruleName===`value_list`?n.push(...$(t)):n.push(t)}}return Pp(e,n),e}_expandValue(e,t){let n=$(e);if(n.length===0)return e;if(n.length===1&&!Z(n[0])){let r=n[0];if(Q(r)===`VARIABLE`){let n=this._substituteVariable(r,t);return Z(n)&&n.ruleName===`value_list`?n:(Pp(e,[n]),e)}}return this._expandChildren(e,t)}_expandFunctionCall(e,t){let n=$(e),r;for(let e of n)if(!Z(e)&&Q(e)===`FUNCTION`){r=e.value.replace(/\($/,``);break}return r===void 0?this._expandChildren(e,t):this.functions.has(r)?this._evaluateFunctionCall(r,e,t):Tp(r)&&!yp.has(r)?this._expandChildren(e,t):yp.has(r)?this._evaluateBuiltinFunction(r,e,t):(Tp(r),this._expandChildren(e,t))}_expandInclude(e,t){let n=$(e),r,i,a,o=null;for(let e of n)if(Z(e)){let t=e;t.ruleName===`include_args`?a=t:t.ruleName===`block`&&(o=t)}else{let t=e,n=Q(t);n===`FUNCTION`?(r=t.value.replace(/\($/,``),i=t):n===`IDENT`&&(r=t.value,i=t)}if(r===void 0)return[];if(!this.mixins.has(r))throw new nf(r,i?.line??0,i?.column??0,[...this.mixins.keys()]);if(this.mixinStack.includes(r))throw new af(`mixin`,[...this.mixinStack,r]);let s=this.mixins.get(r),{positional:c,named:l}=a?this._parseIncludeArgs(a):{positional:[],named:new Map},u=c.length+l.size;if(u<s.params.length-s.defaults.size||u>s.params.length)throw new rf(`Mixin`,r,s.params.length,u);let d=e=>{let n=Np(e),r=this._expandNode(n,t);return r===null?e:Array.isArray(r)?r[0]??e:r},f=t.child(),p=0;for(let e=0;e<s.params.length;e++){let t=s.params[e];l.has(t)?f.set(t,d(l.get(t))):p<c.length?f.set(t,d(c[p++])):s.defaults.has(t)&&f.set(t,Np(s.defaults.get(t)))}this.contentBlockStack.push(o),this.contentScopeStack.push(t),this.mixinStack.push(r);try{let e=Np(s.body),t=this._expandNode(e,f),n=Array.isArray(t)?t[0]:t;if(n&&Z(n)){for(let e of $(n))if(Z(e)&&e.ruleName===`block_contents`)return $(e).filter(e=>e!==null)}return[]}finally{this.mixinStack.pop(),this.contentBlockStack.pop(),this.contentScopeStack.pop()}}_parseIncludeArgs(e){let t=[],n=new Map;for(let r of $(e)){if(!Z(r))continue;let e=r;if(e.ruleName===`include_arg`){let r=$(e);if(r.length>=3&&!Z(r[0])&&Q(r[0])===`VARIABLE`&&!Z(r[1])&&Q(r[1])===`COLON`){let e=r[0].value,t=r[2];n.set(e,t)}else{let e=r.find(e=>Z(e)&&e.ruleName===`value_list`);e&&t.push(e)}}else e.ruleName===`value_list`&&t.push(e)}if(t.length===1&&n.size===0){let e=this._splitValueListOnCommas(t[0]);if(e.length>1)return{positional:e,named:n}}return{positional:t,named:n}}_splitValueListOnCommas(e){let t=$(e),n=!1;for(let e of t)if(Z(e)&&e.ruleName===`value`){for(let t of $(e))if(!Z(t)&&Q(t)===`COMMA`){n=!0;break}}if(!n)return[e];let r=[[]];for(let e of t){if(Z(e)&&e.ruleName===`value`){let t=$(e);if(t.length===1&&!Z(t[0])&&Q(t[0])===`COMMA`){r.push([]);continue}}r[r.length-1].push(e)}return r.filter(e=>e.length>0).map(e=>new Dp(`value_list`,e))}_expandControl(e,t){let n=$(e);if(n.length===0)return null;let r=n[0];if(!Z(r))return null;let i=r;switch(i.ruleName){case`if_directive`:return this._expandIf(i,t);case`for_directive`:return this._expandFor(i,t);case`each_directive`:return this._expandEach(i,t);case`while_directive`:return this._expandWhile(i,t)}return null}_expandIf(e,t){let n=$(e),r=[],i=0;for(;i<n.length;){let e=n[i],t=Cp(e);if(t===`@if`){let e=n[i+1],t=n[i+2];t&&Z(t)&&r.push({condition:e,block:t}),i+=3}else if(t===`@else`)if(i+1<n.length&&Cp(n[i+1])===`if`){let e=n[i+2],t=n[i+3];t&&Z(t)&&r.push({condition:e,block:t}),i+=4}else{let e=n[i+1];e&&Z(e)&&r.push({condition:null,block:e}),i+=2}else i++}let a=new Sp(t);for(let{condition:e,block:n}of r)if(e===null)return this._expandBlockToItems(n,t);else if(Cf(a.evaluate(e)))return this._expandBlockToItems(n,t);return[]}_expandFor(e,t){let n=$(e),r,i,a,o=!1,s,c=0;for(;c<n.length;){let e=n[c],t=Cp(e);t!==void 0&&!Z(e)&&Q(e)===`VARIABLE`?r=t:t===`from`?(i=n[c+1],c++):t===`through`?(o=!0,a=n[c+1],c++):t===`to`?(o=!1,a=n[c+1],c++):Z(e)&&e.ruleName===`block`&&(s=e),c++}if(!r||!i||!a||!s)return[];let l=new Sp(t),u=l.evaluate(i),d=l.evaluate(a),f=u.kind===`number`?Math.trunc(u.value):0,p=d.kind===`number`?Math.trunc(d.value):0,m=o?p+1:p,h=[];for(let e=f;e<m;e++){let n=t.child();n.set(r,new pf(e));let i=this._expandBlockToItems(Np(s),n);h.push(...i)}return h}_expandEach(e,t){let n=$(e),r=[],i,a;for(let e of n)if(Z(e)){let t=e;t.ruleName===`each_list`?i=t:t.ruleName===`block`&&(a=t)}else{let t=e;Q(t)===`VARIABLE`&&r.push(t.value)}if(r.length===0||!i||!a)return[];let o=this._resolveEachList(i,t);if(o!==null)return this._expandEachOverResolved(r,o,a,t);let s=[];for(let e of $(i))Z(e)&&e.ruleName===`value`&&s.push(e);let c=[];for(let e of s){let n=t.child();if(r.length>0){let t=this._extractValueToken(e);n.set(r[0],t)}let i=this._expandBlockToItems(Np(a),n);c.push(...i)}return c}_resolveEachList(e,t){let n=[];for(let t of $(e))if(Z(t)&&t.ruleName===`value`)for(let e of $(t))!Z(e)&&Q(e)===`VARIABLE`&&n.push(e);if(n.length===1){let e=t.get(n[0].value);if(typeof e==`object`&&e&&`kind`in e){let t=e;if(t.kind===`map`||t.kind===`list`)return t}if(typeof e==`object`&&e&&`ruleName`in e){let n=this._findMapLiteralInAst(e);if(n)return this._convertMapLiteralToLatticeMap(n,t)}}return null}_findMapLiteralInAst(e){if(e.ruleName===`map_literal`)return e;for(let t of $(e))if(Z(t)){let e=this._findMapLiteralInAst(t);if(e)return e}return null}_convertMapLiteralToLatticeMap(e,t){let n=[],r=new Sp(t);for(let t of $(e)){if(!Z(t)||t.ruleName!==`map_entry`)continue;let e,i;for(let n of $(t))if(Z(n)){let e=n;e.ruleName===`lattice_expression`&&i===void 0&&(i=e)}else{let t=n,r=Q(t);(r===`IDENT`||r===`STRING`)&&e===void 0&&(e=t.value.replace(/^"|"$/g,``).replace(/^'|'$/g,``))}if(e!==void 0&&i!==void 0){let t=r.evaluate(i);n.push([e,t])}}return new Sf(n)}_expandEachOverResolved(e,t,n,r){let i=[];if(t.kind===`map`)for(let[a,o]of t.items){let t=r.child();t.set(e[0],new _f(a)),e.length>=2&&t.set(e[1],o),i.push(...this._expandBlockToItems(Np(n),t))}else if(t.kind===`list`)for(let a of t.items){let t=r.child();t.set(e[0],a),i.push(...this._expandBlockToItems(Np(n),t))}return i}_extractValueToken(e){if(Z(e)){let t=$(e);if(t.length===1&&!Z(t[0]))return wf(t[0])}return e}_expandBlockToItems(e,t){let n=this._expandNode(e,t),r=Array.isArray(n)?n[0]:n;if(r&&Z(r)){for(let e of $(r))if(Z(e)&&e.ruleName===`block_contents`)return $(e).filter(e=>e!==null)}return[]}_evaluateFunctionCall(e,t,n){let r=this.functions.get(e),i=$(t),a=[];for(let e of i)if(Z(e)&&e.ruleName===`function_args`){a=this._parseFunctionCallArgs(e,n);break}let o=r.params.length-r.defaults.size;if(a.length<o||a.length>r.params.length)throw new rf(`Function`,e,r.params.length,a.length);if(this.functionStack.includes(e))throw new af(`function`,[...this.functionStack,e]);let s=this.variables.child();for(let e=0;e<r.params.length;e++){let t=r.params[e];e<a.length?s.set(t,a[e]):r.defaults.has(t)&&s.set(t,Np(r.defaults.get(t)))}this.functionStack.push(e);try{let n=Np(r.body);try{this._evaluateFunctionBody(n,s)}catch(e){if(e instanceof Ep)return Mp(Ef(e.value),t);throw e}throw new of(e)}finally{this.functionStack.pop()}}_evaluateFunctionBody(e,t){if(Z(e))for(let n of $(e)){if(!Z(n))continue;let e=n;if(e.ruleName===`function_body_item`){let n=$(e);if(n.length===0)continue;let r=n[0];if(!Z(r))continue;let i=r;i.ruleName===`variable_declaration`?this._expandVariableDeclaration(i,t):i.ruleName===`return_directive`?this._evaluateReturn(i,t):i.ruleName===`lattice_control`&&this._evaluateControlInFunction(i,t)}else this._evaluateFunctionBody(e,t)}}_evaluateReturn(e,t){for(let n of $(e))if(Z(n)&&n.ruleName===`lattice_expression`)throw new Ep(new Sp(t).evaluate(n));throw new Ep(new bf)}_evaluateControlInFunction(e,t){let n=$(e);if(n.length===0)return;let r=n[0];if(!Z(r))return;let i=r;i.ruleName===`if_directive`&&this._evaluateIfInFunction(i,t)}_evaluateIfInFunction(e,t){let n=$(e),r=[],i=0;for(;i<n.length;){let e=n[i],t=Cp(e);if(t===`@if`){let e=n[i+1],t=n[i+2];t&&Z(t)&&r.push({condition:e,block:t}),i+=3}else if(t===`@else`)if(i+1<n.length&&Cp(n[i+1])===`if`){let e=n[i+2],t=n[i+3];t&&Z(t)&&r.push({condition:e,block:t}),i+=4}else{let e=n[i+1];e&&Z(e)&&r.push({condition:null,block:e}),i+=2}else i++}let a=new Sp(t);for(let{condition:e,block:n}of r)if(e===null||Cf(a.evaluate(e))){this._evaluateBlockInFunction(n,t);return}}_evaluateBlockInFunction(e,t){if(Z(e))for(let n of $(e)){if(!Z(n))continue;let e=n;if(e.ruleName===`block_contents`)this._evaluateBlockInFunction(e,t);else if(e.ruleName===`block_item`){let n=$(e);if(n.length>0&&Z(n[0])){let e=n[0];if(e.ruleName===`at_rule`)this._maybeEvaluateReturnAtRule(e,t);else if(e.ruleName===`lattice_block_item`)for(let n of $(e))Z(n)&&n.ruleName===`variable_declaration`&&this._expandVariableDeclaration(n,t)}}}}_maybeEvaluateReturnAtRule(e,t){let n,r;for(let t of $(e))Z(t)?t.ruleName===`at_prelude`&&(r=t):Q(t)===`AT_KEYWORD`&&(n=t.value);if(n!==`@return`||!r)return;let i=[];if(this._collectTokens(r,i),i.length===0)throw new Ep(new bf);if(i.length===1){let e=i[0];if(Q(e)===`VARIABLE`){let n=t.get(e.value);if(n!==void 0){if(typeof n==`object`&&n&&`kind`in n)throw new Ep(n);if(typeof n==`object`&&n&&`ruleName`in n)throw new Ep(Tf(n))}}throw new Ep(wf(e))}throw new Ep(wf(i[0]))}_collectTokens(e,t){if(!Z(e)){t.push(e);return}for(let n of $(e))this._collectTokens(n,t)}_parseFunctionCallArgs(e,t){let n=[[]];for(let r of $(e)){if(!Z(r)&&Q(r)===`COMMA`){n.push([]);continue}if(Z(r)&&r.ruleName===`function_arg`)for(let e of $(r))if(Z(e))n[n.length-1].push(e);else{if(Q(e)===`COMMA`){n.push([]);continue}let r=e;if(t&&Q(r)===`VARIABLE`){let e=t.get(r.value);e==null?n[n.length-1].push(wf(r)):typeof e==`object`&&`kind`in e?n[n.length-1].push(e):typeof e==`object`&&`type`in e?n[n.length-1].push(wf(e)):typeof e==`object`&&`ruleName`in e?n[n.length-1].push(Tf(e)):n[n.length-1].push(wf(r))}else n[n.length-1].push(wf(r))}}let r=[];for(let e of n)(e.length===1||e.length>1)&&r.push(e[0]);return r}_expandWhile(e,t){let n=$(e),r,i;for(let e of n)if(Z(e)){let t=e;t.ruleName===`lattice_expression`?r=t:t.ruleName===`block`&&(i=t)}if(!r||!i)return[];let a=[],o=0;for(;Cf(new Sp(t).evaluate(Np(r)));){if(o++,o>this.maxWhileIterations)throw new sf(this.maxWhileIterations);let e=this._expandBlockToItems(Np(i),t);a.push(...e)}return a}_expandSelectorWithVars(e,t){let n=[];for(let r of $(e))if(Z(r)){let e=this._expandNode(r,t);e!==null&&(Array.isArray(e)?n.push(...e):n.push(e))}else{let e=r;if(Q(e)===`VARIABLE`){let r=e.value,i=t.get(r);if(i===void 0)throw new tf(r,e.line??0,e.column??0);let a;a=typeof i==`object`&&i&&`kind`in i?Ef(i):typeof i==`object`&&i&&`ruleName`in i?Ef(Tf(i)):String(i),a=a.replace(/^"|"$/g,``).replace(/^'|'$/g,``),n.push(jp(a,e))}else n.push(r)}return Pp(e,n),e}_expandContent(e){if(this.contentBlockStack.length===0)return[];let t=this.contentBlockStack[this.contentBlockStack.length-1];if(t===null)return[];let n=this.contentScopeStack.length>0?this.contentScopeStack[this.contentScopeStack.length-1]:e;return this._expandBlockToItems(Np(t),n)}_expandAtRoot(e,t){let n=$(e),r,i;for(let e of n)if(Z(e)){let t=e;t.ruleName===`block`?r=t:t.ruleName===`selector_list`&&(i=t)}if(!r)return null;if(i){let e=new Dp(`qualified_rule`,[this._expandNode(Np(i),t),this._expandNode(Np(r),t)]);this.atRootRules.push(e)}else{let e=this._expandBlockToItems(Np(r),t);this.atRootRules.push(...e)}return null}_collectExtend(e){let t=``;for(let n of $(e))if(Z(n)&&n.ruleName===`extend_target`){let e=[];for(let t of $(n))Z(t)||e.push(t.value);t=e.join(``)}t&&(this.extendMap.has(t)||this.extendMap.set(t,[]))}_expandPropertyNesting(e,t){let n=``,r;for(let t of $(e))if(Z(t)){let e=t;if(e.ruleName===`property`)for(let t of $(e))Z(t)||(n=t.value);else e.ruleName===`block`&&(r=e)}if(!n||!r)return[];let i=this._expandNode(Np(r),t),a=[];return this._flattenNestedProps(i,n,a),a}_flattenNestedProps(e,t,n){if(Z(e))for(let r of $(e)){if(!Z(r))continue;let e=r;e.ruleName===`block_contents`?this._flattenNestedProps(e,t,n):e.ruleName===`block_item`?this._flattenNestedBlockItem(e,t,n):e.ruleName===`declaration`&&this._rewriteDeclarationPrefix(e,t,n)}}_flattenNestedBlockItem(e,t,n){let r=$(e);if(r.length===0)return;let i=r[0];if(!Z(i))return;let a=i;if(a.ruleName===`declaration_or_nested`){for(let e of $(a))if(Z(e)){let r=e;if(r.ruleName===`declaration`)this._rewriteDeclarationPrefix(r,t,n);else if(r.ruleName===`property_nesting`){let e=this._expandPropertyNestingWithPrefix(r,t);n.push(...e)}}}}_rewriteDeclarationPrefix(e,t,n){for(let n of $(e))if(Z(n)&&n.ruleName===`property`){for(let e of $(n))if(!Z(e)){let n=e;n.value=`${t}-${n.value}`}}n.push(e)}_expandPropertyNestingWithPrefix(e,t){let n=``,r;for(let t of $(e))if(Z(t)){let e=t;if(e.ruleName===`property`)for(let t of $(e))Z(t)||(n=t.value);else e.ruleName===`block`&&(r=e)}let i=`${t}-${n}`,a=[];return r&&this._flattenNestedProps(r,i,a),a}_evaluateBuiltinFunction(e,t,n){let r=$(t),i=[];for(let e of r)if(Z(e)&&e.ruleName===`function_args`){let t=new Sp(n);i=this._collectBuiltinFunctionArgs(e,t);break}let a=yp.get(e)(i);return a.kind===`null`?this._expandChildren(t,n):Mp(Ef(a),t)}_collectBuiltinFunctionArgs(e,t){let n=[],r=[],i=()=>{if(r.length>0){if(r.length===1){let e=r[0];if(Q(e)===`VARIABLE`){let r=t.scope.get(e.value);typeof r==`object`&&r&&`kind`in r?n.push(r):typeof r==`object`&&r&&`ruleName`in r?n.push(Tf(r)):n.push(wf(e))}else n.push(wf(e))}else n.push(wf(r[0]));r.length=0}};for(let a of $(e)){if(!Z(a)&&Q(a)===`COMMA`){i();continue}if(Z(a)&&a.ruleName===`function_arg`)for(let e of $(a))if(Z(e))n.push(t.evaluate(e)),r.length=0;else{if(Q(e)===`COMMA`){i();continue}r.push(e)}}return i(),n}_applyExtends(e){this._removePlaceholderRules(e)}_removePlaceholderRules(e){if(!Z(e))return;let t=[];for(let n of $(e))n!==null&&(this._isPlaceholderOnlyRule(n)||(this._removePlaceholderRules(n),t.push(n)));Pp(e,t)}_isPlaceholderOnlyRule(e){if(!Z(e))return!1;let t=e;if(t.ruleName===`qualified_rule`){let e=this._extractSelectorText(t).split(`,`).map(e=>e.trim()).filter(e=>e);return e.length>0&&e.every(e=>e.startsWith(`%`))}if(t.ruleName===`rule`){let e=$(t);if(e.length>0&&Z(e[0]))return this._isPlaceholderOnlyRule(e[0])}return!1}_extractSelectorText(e){for(let t of $(e))if(Z(t)&&t.ruleName===`selector_list`)return this._collectText(t);return``}_collectText(e){if(!Z(e))return e.value;let t=[];for(let n of $(e))t.push(this._collectText(n));return t.join(` `)}_spliceAtRootRules(e){for(let t of this.atRootRules)t!==null&&$(e).push(t)}_cleanup(e){if(!Z(e))return e;let t=e,n=[];for(let e of $(t)){if(e===null)continue;let t=this._cleanup(e);t!==null&&n.push(t)}return Pp(t,n),t}};function Ip(e){return`ruleName`in e}function Lp(e){return e.type}var Rp=class{indent;minified;constructor(e=`  `,t=!1){this.indent=e,this.minified=t}emit(e){let t=this._emitNode(e,0).trim();return t?t+`
`:``}_emitNode(e,t){if(!Ip(e))return e.value;let n=e;switch(n.ruleName){case`stylesheet`:return this._emitStylesheet(n,t);case`rule`:return this._emitRule(n,t);case`qualified_rule`:return this._emitQualifiedRule(n,t);case`at_rule`:return this._emitAtRule(n,t);case`at_prelude`:return this._emitAtPrelude(n,t);case`at_prelude_token`:return this._emitDefault(n,t);case`at_prelude_tokens`:return this._emitAtPreludeTokens(n,t);case`function_in_prelude`:return this._emitFunctionInPrelude(n,t);case`paren_block`:return this._emitParenBlock(n,t);case`selector_list`:return this._emitSelectorList(n,t);case`complex_selector`:return this._emitComplexSelector(n,t);case`combinator`:return this._emitCombinator(n,t);case`compound_selector`:return this._emitCompoundSelector(n,t);case`simple_selector`:return this._emitSimpleSelector(n,t);case`subclass_selector`:return this._emitSubclassSelector(n,t);case`class_selector`:return this._emitClassSelector(n,t);case`id_selector`:return this._emitIdSelector(n,t);case`attribute_selector`:return this._emitAttributeSelector(n,t);case`attr_matcher`:return this._emitAttrMatcher(n,t);case`attr_value`:return this._emitAttrValue(n,t);case`pseudo_class`:return this._emitPseudoClass(n,t);case`pseudo_class_args`:return this._emitPseudoClassArgs(n,t);case`pseudo_class_arg`:return this._emitDefault(n,t);case`pseudo_element`:return this._emitPseudoElement(n,t);case`block`:return this._emitBlock(n,t);case`block_contents`:return this._emitBlockContents(n,t);case`block_item`:return this._emitBlockItem(n,t);case`declaration_or_nested`:return this._emitDeclarationOrNested(n,t);case`declaration`:return this._emitDeclaration(n,t);case`property`:return this._emitProperty(n,t);case`priority`:return this._emitPriority(n,t);case`value_list`:return this._emitValueList(n,t);case`value`:return this._emitValue(n,t);case`function_call`:return this._emitFunctionCall(n,t);case`function_args`:return this._emitFunctionArgs(n,t);case`function_arg`:return this._emitFunctionArg(n,t);default:return this._emitDefault(n,t)}}_emitStylesheet(e,t){let n=[];for(let r of e.children){let e=this._emitNode(r,t);e.trim()&&n.push(e)}return this.minified?n.join(``):n.join(`

`)}_emitRule(e,t){let n=e.children;return n.length>0?this._emitNode(n[0],t):``}_emitQualifiedRule(e,t){let n=``,r=``;for(let i of e.children){if(!Ip(i))continue;let e=i;if(e.ruleName===`selector_list`)n=this._emitNode(i,t);else if(e.ruleName===`block`)r=this._emitBlock(e,t);else{let e=this._emitNode(i,t);e.trim()&&(n+=e)}}return this.minified?`${n}${r}`:n?`${n} ${r}`:r}_emitAtRule(e,t){let n=``,r=``,i=``,a=!1;for(let o of e.children)if(Ip(o)){let e=o;e.ruleName===`at_prelude`?r=this._emitAtPrelude(e,t):e.ruleName===`block`&&(i=this._emitBlock(e,t))}else{let e=o,t=Lp(e);t===`AT_KEYWORD`?n=e.value:t===`SEMICOLON`&&(a=!0)}if(this.minified)return a?`${n}${r};`:`${n}${r}${i}`;if(a){let e=r.trim()?` ${r.trim()}`:``;return`${n}${e};`}let o=r.trim()?` ${r.trim()}`:``;return`${n}${o} ${i}`}_emitAtPrelude(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}_emitAtPreludeTokens(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}_emitFunctionInPrelude(e,t){let n=[];for(let r of e.children)if(Ip(r))n.push(this._emitNode(r,t));else{let e=r;Lp(e)===`RPAREN`?n.push(`)`):n.push(e.value)}return n.join(``)}_emitParenBlock(e,t){let n=[];for(let r of e.children)if(Ip(r))n.push(this._emitNode(r,t));else{let e=r,t=Lp(e);t===`LPAREN`?n.push(`(`):t===`RPAREN`?n.push(`)`):n.push(e.value)}return n.join(``)}_emitSelectorList(e,t){let n=[];for(let r of e.children)Ip(r)&&n.push(this._emitNode(r,t));let r=this.minified?`,`:`, `;return n.join(r)}_emitComplexSelector(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}_emitCombinator(e,t){return e.children.length>0?e.children[0].value:``}_emitCompoundSelector(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(``)}_emitSimpleSelector(e,t){return e.children.length>0?e.children[0].value:``}_emitSubclassSelector(e,t){return e.children.length>0?this._emitNode(e.children[0],t):``}_emitClassSelector(e,t){let n=[];for(let t of e.children)Ip(t)||n.push(t.value);return n.join(``)}_emitIdSelector(e,t){return e.children.length>0?e.children[0].value:``}_emitAttributeSelector(e,t){let n=[];for(let r of e.children)if(Ip(r))n.push(this._emitNode(r,t));else{let e=r,t=Lp(e);t===`LBRACKET`?n.push(`[`):t===`RBRACKET`?n.push(`]`):n.push(e.value)}return n.join(``)}_emitAttrMatcher(e,t){return e.children.length>0?e.children[0].value:``}_emitAttrValue(e,t){if(e.children.length>0){let t=e.children[0];return Lp(t)===`STRING`?`"${t.value}"`:t.value}return``}_emitPseudoClass(e,t){let n=[];for(let r of e.children)if(Ip(r))n.push(this._emitNode(r,t));else{let e=r,t=Lp(e);t===`COLON`?n.push(`:`):t===`RPAREN`?n.push(`)`):n.push(e.value)}return n.join(``)}_emitPseudoClassArgs(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(``)}_emitPseudoElement(e,t){let n=[];for(let t of e.children)if(!Ip(t)){let e=t;Lp(e)===`COLON_COLON`?n.push(`::`):n.push(e.value)}return n.join(``)}_emitBlock(e,t){let n;for(let t of e.children)if(Ip(t)&&t.ruleName===`block_contents`){n=t;break}if(this.minified)return n?`{`+this._emitBlockContents(n,t+1)+`}`:`{}`;if(!n)return`{
`+this.indent.repeat(t)+`}`;let r=this._emitBlockContents(n,t+1);return r.trim()?`{
`+r+`
`+this.indent.repeat(t)+`}`:`{
`+this.indent.repeat(t)+`}`}_emitBlockContents(e,t){let n=[];for(let r of e.children){let e=this._emitNode(r,t);e.trim()&&n.push(e)}if(this.minified)return n.join(``);let r=this.indent.repeat(t);return n.map(e=>`${r}${e}`).join(`
`)}_emitBlockItem(e,t){return e.children.length>0?this._emitNode(e.children[0],t):``}_emitDeclarationOrNested(e,t){return e.children.length>0?this._emitNode(e.children[0],t):``}_emitDeclaration(e,t){let n=``,r=``,i=``;for(let t of e.children){if(!Ip(t))continue;let e=t;e.ruleName===`property`?n=this._emitProperty(e,0):e.ruleName===`value_list`?r=this._emitValueList(e,0):e.ruleName===`priority`&&(i=` !important`)}return this.minified?`${n}:${r}${i};`:`${n}: ${r}${i};`}_emitProperty(e,t){return e.children.length>0?e.children[0].value:``}_emitPriority(e,t){return`!important`}_emitValueList(e,t){let n=[];for(let r of e.children){let e=this._emitNode(r,t);n.push(e)}let r=n.join(` `);return r=r.replace(/ , /g,`, `).replace(/ ,/g,`,`),r}_emitValue(e,t){let n=e.children;if(n.length===1){let e=n[0];if(!Ip(e)){let t=e;return Lp(t)===`STRING`?`"${t.value}"`:t.value}return this._emitNode(e,t)}return this._emitDefault(e,t)}_emitFunctionCall(e,t){let n=e.children;if(n.length===1)return n[0].value;let r=[];for(let e of n)if(Ip(e))r.push(this._emitNode(e,t));else{let t=e,n=Lp(t);n===`FUNCTION`?r.push(t.value):n===`RPAREN`?r.push(`)`):r.push(t.value)}return r.join(``)}_emitFunctionArgs(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));let r=n.join(` `);return r=r.replace(/ , /g,`, `).replace(/ ,/g,`,`),r}_emitFunctionArg(e,t){let n=e.children;if(n.length===1){let e=n[0];return Ip(e)?this._emitNode(e,t):e.value}let r=[];for(let e of n)if(Ip(e))r.push(this._emitNode(e,t));else{let t=e;r.push(Lp(t)===`RPAREN`?`)`:t.value)}return r.join(``)}_emitDefault(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}};function zp(e,t={}){let n=Zd(e),r=new Fp().transform(n);return new Rp(t.indent??`  `,t.minified??!1).emit(r)}function Bp(e,t={}){return zp(e,t)}var Vp=`$paper: #f7f8f3;
$ink: #17201c;
$muted: #5d6d68;
$panel: rgba(255, 255, 255, 0.92);
$line: rgba(23, 32, 28, 0.14);
$green: #237a57;
$blue: #2563eb;
$red: #c2413b;
$gold: #b7791f;
$violet: #6d5bd0;

@mixin surface() {
  border: 1px solid $line;
  background: $panel;
  border-radius: 8px;
  box-shadow: 0 18px 50px rgba(29, 45, 39, 0.08);
}

.workspace--initialization {
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.initialization-stage {
  display: grid;
  gap: 16px;
  min-width: 0;
}

.initialization-intro,
.distribution-summary-panel,
.initialization-arithmetic,
.initializer-comparison,
.initialization-controls {
  @include surface;
}

.initialization-intro,
.distribution-summary-panel,
.initialization-arithmetic,
.initializer-comparison {
  padding: 18px;
}

.initialization-intro {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.initialization-intro h2,
.initialization-controls h2 {
  margin: 0;
}

.initialization-intro p:not(.eyebrow),
.initialization-controls p,
.initializer-comparison .panel-heading > span {
  color: $muted;
  line-height: 1.55;
}

.initialization-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(109, 91, 208, 0.3);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
  padding: 8px 12px;
  font-weight: 850;
  text-transform: capitalize;
}

.initialization-flow {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.distribution-card {
  display: grid;
  gap: 8px;
  min-width: 0;
  min-height: 132px;
  border: 1px solid $line;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.86);
  color: $ink;
  padding: 12px;
  text-align: left;
}

button.distribution-card:hover,
button.distribution-card[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.09);
}

.distribution-card--input {
  align-content: start;
  border-color: rgba(35, 122, 87, 0.25);
  background: rgba(35, 122, 87, 0.055);
}

.distribution-card strong,
.distribution-card code,
.distribution-stat-grid strong,
.spread-row code,
.initialization-equation {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.distribution-card > span:last-child {
  color: $muted;
  font-size: 0.72rem;
}

.distribution-dot-plot {
  position: relative;
  display: block;
  height: 30px;
  border-radius: 6px;
  background:
    linear-gradient(90deg, transparent 49.5%, rgba(23, 32, 28, 0.14) 49.5%, rgba(23, 32, 28, 0.14) 50.5%, transparent 50.5%),
    rgba(37, 99, 235, 0.055);
  overflow: hidden;
}

.distribution-dot-plot i {
  position: absolute;
  top: 8px;
  width: 9px;
  height: 9px;
  margin-left: -4px;
  border: 2px solid white;
  border-radius: 50%;
  background: $blue;
  box-shadow: 0 0 0 1px rgba(37, 99, 235, 0.28);
}

.distribution-summary-panel,
.initialization-arithmetic,
.initializer-comparison {
  display: grid;
  gap: 14px;
}

.distribution-stat-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.distribution-stat-grid > div {
  display: grid;
  gap: 4px;
  min-width: 0;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.065);
  padding: 10px;
}

.distribution-stat-grid span {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 850;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.activation-value-grid {
  display: grid;
  grid-template-columns: repeat(8, minmax(0, 1fr));
  gap: 6px;
}

.activation-value-grid code {
  min-width: 0;
  overflow: hidden;
  padding: 7px 3px;
  border-radius: 6px;
  background: rgba(37, 99, 235, 0.07);
  color: #234c9f;
  font-size: 0.7rem;
  text-align: center;
  text-overflow: ellipsis;
}

.initialization-equation {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.initialization-equation code,
.initialization-equation strong {
  min-width: 0;
  border-radius: 8px;
  padding: 10px;
}

.initialization-equation code {
  background: rgba(37, 99, 235, 0.065);
  color: #234c9f;
}

.initialization-equation strong {
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
}

.initializer-comparison-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 9px;
}

.initializer-comparison-grid article {
  display: grid;
  gap: 8px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  padding: 11px;
}

.initializer-comparison-grid article.is-selected {
  border-color: rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.055);
}

.spread-row {
  display: grid;
  grid-template-columns: 20px minmax(30px, 1fr) auto;
  align-items: center;
  gap: 6px;
}

.spread-row span,
.spread-row code {
  color: $muted;
  font-size: 0.68rem;
}

.spread-row i {
  display: block;
  min-width: 2px;
  height: 7px;
  border-radius: 999px;
  background: linear-gradient(90deg, $green, $blue);
}

.initialization-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 16px;
  padding: 16px;
}

.initialization-controls section {
  display: grid;
  gap: 9px;
}

.initialization-controls section + section {
  padding-top: 14px;
  border-top: 1px solid $line;
}

.initializer-buttons,
.activation-choice-grid {
  display: grid;
  gap: 7px;
}

.initializer-buttons button,
.activation-choice-grid button {
  display: grid;
  gap: 3px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.initializer-buttons button[aria-pressed="true"],
.activation-choice-grid button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.48);
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
}

.initializer-buttons small {
  color: $muted;
}

.activation-choice-grid {
  grid-template-columns: 1fr 1fr;
}

.activation-choice-grid button {
  text-align: center;
}

.initialization-reading {
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.065);
  padding: 11px;
}

@media (max-width: 1180px) {
  .workspace--initialization {
    grid-template-columns: 1fr;
  }

  .initialization-controls {
    position: static;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .initialization-controls section + section {
    padding-top: 0;
    padding-left: 14px;
    border-top: 0;
    border-left: 1px solid $line;
  }
}

@media (max-width: 820px) {
  .initialization-intro,
  .distribution-summary-panel .panel-heading,
  .initialization-arithmetic .panel-heading,
  .initializer-comparison .panel-heading {
    display: grid;
  }

  .initialization-chip {
    justify-self: start;
  }

  .initialization-flow,
  .distribution-stat-grid,
  .initializer-comparison-grid,
  .initialization-controls {
    grid-template-columns: 1fr;
  }

  .activation-value-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .initialization-equation {
    grid-template-columns: 1fr;
  }

  .initialization-controls section + section {
    padding-top: 14px;
    padding-left: 0;
    border-top: 1px solid $line;
    border-left: 0;
  }
}

.workspace--forward-lowering {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.forward-lowering-stage {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.forward-lowering-intro,
.forward-lowering-graph,
.forward-lowering-ir,
.forward-lowering-selection,
.forward-lowering-parity,
.forward-lowering-controls {
  @include surface;
}

.forward-lowering-intro,
.forward-lowering-graph,
.forward-lowering-ir,
.forward-lowering-selection,
.forward-lowering-parity {
  padding: 18px;
}

.forward-lowering-intro {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.forward-lowering-intro h2,
.forward-lowering-controls h2,
.forward-lowering-graph h2,
.forward-lowering-ir h2,
.forward-lowering-selection h2,
.forward-lowering-parity h2 {
  margin: 0;
}

.forward-lowering-intro p:not(.eyebrow),
.forward-lowering-controls > p {
  color: $muted;
  line-height: 1.55;
}

.forward-lowering-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(109, 91, 208, 0.38);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.08);
  color: #4b3e9d;
  padding: 8px 12px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.72rem;
  font-weight: 850;
}

.forward-lowering-graph,
.forward-lowering-ir,
.forward-lowering-selection,
.forward-lowering-parity {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.forward-lowering-graph .panel-heading > code,
.forward-lowering-ir .panel-heading > code,
.forward-lowering-selection .panel-heading > code,
.forward-lowering-parity .panel-heading > code {
  border-radius: 999px;
  background: rgba(37, 99, 235, 0.07);
  color: #234c9f;
  padding: 7px 10px;
  font-size: 0.72rem;
  overflow-wrap: anywhere;
}

.forward-lowering-node-flow {
  display: grid;
  grid-template-columns: minmax(180px, 0.8fr) auto repeat(3, minmax(120px, 0.65fr));
  align-items: center;
  gap: 8px;
}

.forward-lowering-input-stack,
.forward-lowering-flow-tail {
  display: grid;
  gap: 7px;
  min-width: 0;
}

.forward-lowering-flow-tail {
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
}

.forward-lowering-flow-tail:last-child {
  grid-template-columns: minmax(0, 1fr);
}

.forward-lowering-input-stack article,
.forward-lowering-flow-tail article {
  display: grid;
  gap: 4px;
  min-width: 0;
  border: 1px solid rgba(37, 99, 235, 0.22);
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.045);
  padding: 10px;
}

.forward-lowering-input-stack span,
.forward-lowering-flow-tail span,
.forward-lowering-edge-grid span,
.forward-lowering-instruction-lane span,
.forward-lowering-matrix-lane span,
.forward-lowering-scenario-buttons span {
  color: $muted;
  font-size: 0.69rem;
  line-height: 1.35;
}

.forward-lowering-arrow {
  color: rgba(109, 91, 208, 0.65);
  font-size: 1.25rem;
  font-weight: 900;
}

.forward-lowering-edge-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.forward-lowering-edge-grid > div {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 5px 9px;
  min-width: 0;
  border-radius: 8px;
  background: rgba(183, 121, 31, 0.07);
  padding: 10px;
}

.forward-lowering-edge-grid span {
  grid-column: 1 / -1;
}

.forward-lowering-edge-grid code,
.forward-lowering-edge-grid strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  overflow-wrap: anywhere;
}

.forward-lowering-edge-grid code {
  color: #7a5318;
}

.forward-lowering-instruction-lane,
.forward-lowering-matrix-lane {
  display: grid;
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.forward-lowering-instruction-lane {
  grid-template-columns: repeat(12, minmax(142px, 1fr));
}

.forward-lowering-matrix-lane {
  grid-template-columns: repeat(6, minmax(165px, 1fr));
}

.forward-lowering-instruction-lane button,
.forward-lowering-matrix-lane button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.84);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.forward-lowering-instruction-lane button[aria-pressed="true"],
.forward-lowering-matrix-lane button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.52);
  background: rgba(109, 91, 208, 0.09);
  box-shadow: inset 0 -3px rgba(109, 91, 208, 0.14);
}

.forward-lowering-instruction-lane small,
.forward-lowering-matrix-lane small,
.forward-lowering-detail-grid small {
  color: $muted;
  font-size: 0.66rem;
  font-weight: 800;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.forward-lowering-instruction-lane strong,
.forward-lowering-matrix-lane strong {
  overflow-wrap: anywhere;
}

.forward-lowering-instruction-lane code,
.forward-lowering-matrix-lane code,
.forward-lowering-detail-grid code,
.forward-lowering-equation code,
.forward-lowering-parity-table code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.forward-lowering-instruction-lane code,
.forward-lowering-matrix-lane code {
  color: #234c9f;
  font-size: 0.68rem;
}

.forward-lowering-detail-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.forward-lowering-detail-grid > div {
  display: grid;
  gap: 7px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.05);
  padding: 11px;
}

.forward-lowering-detail-grid code {
  color: #1d6849;
  overflow-wrap: anywhere;
}

.forward-lowering-parity-table {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.forward-lowering-parity-table > div {
  display: grid;
  grid-template-columns: repeat(6, minmax(62px, 1fr));
  gap: 6px;
  align-items: center;
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.045);
  padding: 9px;
}

.forward-lowering-parity-table .forward-lowering-parity-head {
  background: rgba(109, 91, 208, 0.08);
  color: #4b3e9d;
  font-size: 0.7rem;
}

.forward-lowering-parity-table code {
  color: #1d6849;
}

.forward-lowering-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 11px;
  padding: 16px;
}

.forward-lowering-scenario-buttons {
  display: grid;
  gap: 7px;
}

.forward-lowering-scenario-buttons button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.forward-lowering-scenario-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.09);
}

.forward-lowering-equation,
.forward-lowering-mental-model {
  display: grid;
  gap: 7px;
  border-radius: 9px;
  padding: 11px;
}

.forward-lowering-equation {
  background: rgba(37, 99, 235, 0.055);
}

.forward-lowering-equation code {
  color: #234c9f;
  overflow-wrap: anywhere;
}

.forward-lowering-mental-model {
  background: rgba(35, 122, 87, 0.065);
}

.forward-lowering-mental-model h2,
.forward-lowering-mental-model p {
  margin: 0;
}

.forward-lowering-mental-model p:not(.eyebrow) {
  color: $muted;
  line-height: 1.45;
}

@media (max-width: 1180px) {
  .workspace--forward-lowering {
    grid-template-columns: 1fr;
  }

  .forward-lowering-controls {
    position: static;
  }

  .forward-lowering-scenario-buttons {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .forward-lowering-intro,
  .forward-lowering-graph .panel-heading,
  .forward-lowering-ir .panel-heading,
  .forward-lowering-selection .panel-heading,
  .forward-lowering-parity .panel-heading {
    display: grid;
  }

  .forward-lowering-chip {
    justify-self: start;
  }

  .forward-lowering-node-flow,
  .forward-lowering-edge-grid,
  .forward-lowering-detail-grid,
  .forward-lowering-scenario-buttons {
    grid-template-columns: 1fr;
  }

  .forward-lowering-flow-tail {
    grid-template-columns: 1fr;
  }

  .forward-lowering-node-flow > .forward-lowering-arrow,
  .forward-lowering-flow-tail > .forward-lowering-arrow {
    transform: rotate(90deg);
    justify-self: center;
  }

  .forward-lowering-parity-table {
    overflow-x: auto;
  }

  .forward-lowering-parity-table > div {
    min-width: 570px;
  }
}

.workspace--backend-parity {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.backend-parity-stage {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.backend-parity-intro,
.backend-parity-paper,
.backend-parity-lanes,
.backend-parity-inspector,
.backend-parity-results,
.backend-parity-probe,
.backend-parity-controls {
  @include surface;
}

.backend-parity-intro,
.backend-parity-paper,
.backend-parity-lanes,
.backend-parity-inspector,
.backend-parity-results,
.backend-parity-probe {
  padding: 18px;
}

.backend-parity-intro,
.backend-parity-probe {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.backend-parity-intro p:not(.eyebrow),
.backend-parity-probe p,
.backend-parity-controls > p,
.backend-parity-controls section p:not(.eyebrow) {
  color: $muted;
  line-height: 1.5;
}

.backend-parity-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(35, 122, 87, 0.34);
  border-radius: 999px;
  background: rgba(35, 122, 87, 0.08);
  color: #1d6849;
  padding: 8px 12px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.72rem;
  font-weight: 850;
}

.backend-parity-paper,
.backend-parity-lanes,
.backend-parity-inspector,
.backend-parity-results {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.backend-parity-paper .panel-heading > code,
.backend-parity-lanes .panel-heading > code,
.backend-parity-inspector .panel-heading > code,
.backend-parity-results .panel-heading > code {
  border-radius: 999px;
  background: rgba(37, 99, 235, 0.07);
  color: #234c9f;
  padding: 7px 10px;
  font-size: 0.72rem;
  overflow-wrap: anywhere;
}

.backend-parity-equation-flow {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  flex-wrap: wrap;
  border-radius: 10px;
  background: rgba(183, 121, 31, 0.07);
  padding: 14px;
}

.backend-parity-equation-flow code,
.backend-parity-equation-flow strong {
  padding: 8px 10px;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.backend-parity-equation-flow strong {
  border-radius: 7px;
  background: rgba(35, 122, 87, 0.13);
  color: #1d6849;
}

.backend-parity-equation-flow span {
  color: $muted;
  font-weight: 900;
}

.backend-parity-paper-table,
.backend-parity-results-table {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.backend-parity-paper-table > div,
.backend-parity-results-table > div {
  display: grid;
  align-items: center;
  gap: 7px;
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.045);
  padding: 9px;
}

.backend-parity-paper-table > div {
  grid-template-columns: repeat(4, minmax(70px, 1fr));
}

.backend-parity-results-table > div {
  grid-template-columns: minmax(175px, 1.5fr) repeat(4, minmax(68px, 0.7fr));
}

.backend-parity-table-head {
  background: rgba(109, 91, 208, 0.08) !important;
  color: #4b3e9d;
  font-size: 0.7rem;
}

.backend-parity-paper-table code,
.backend-parity-results-table code,
.backend-parity-lane-grid code,
.backend-parity-inspector code,
.backend-parity-probe code,
.backend-parity-controls code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.backend-parity-lane-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.backend-parity-lane-grid button {
  display: grid;
  align-content: start;
  gap: 7px;
  min-width: 0;
  min-height: 145px;
  border: 1px solid $line;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.84);
  color: $ink;
  padding: 11px;
  text-align: left;
}

.backend-parity-lane-grid button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.52);
  background: rgba(109, 91, 208, 0.09);
  box-shadow: inset 0 -3px rgba(109, 91, 208, 0.14);
}

.backend-parity-lane-grid small {
  color: $muted;
  font-size: 0.64rem;
  font-weight: 800;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.backend-parity-lane-grid span {
  color: $muted;
  font-size: 0.7rem;
  line-height: 1.35;
}

.backend-parity-lane-grid code {
  align-self: end;
  color: #1d6849;
  font-size: 0.72rem;
}

.backend-parity-detail-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 9px;
}

.backend-parity-detail-grid > div {
  display: grid;
  align-content: start;
  gap: 8px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.05);
  padding: 11px;
}

.backend-parity-detail-grid small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 850;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.backend-parity-detail-grid ol {
  display: grid;
  gap: 7px;
  margin: 0;
  padding-left: 21px;
}

.backend-parity-detail-grid li,
.backend-parity-detail-grid p {
  color: $muted;
  line-height: 1.4;
}

.backend-parity-probe {
  align-items: center;
}

.backend-parity-probe > div:first-child {
  min-width: 0;
}

.backend-parity-probe-status {
  display: grid;
  gap: 9px;
  min-width: 180px;
  border: 1px solid $line;
  border-radius: 10px;
  padding: 11px;
  text-align: center;
}

.backend-parity-probe-status > strong {
  color: $muted;
  font-size: 0.72rem;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.backend-parity-probe-status--executed {
  border-color: rgba(35, 122, 87, 0.4);
  background: rgba(35, 122, 87, 0.07);
}

.backend-parity-probe-status--failed {
  border-color: rgba(194, 65, 59, 0.4);
  background: rgba(194, 65, 59, 0.06);
}

.backend-parity-probe-status button:disabled {
  cursor: wait;
  opacity: 0.55;
}

.backend-parity-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 12px;
  padding: 16px;
}

.backend-parity-controls section {
  border-top: 1px solid $line;
  padding-top: 12px;
}

.backend-parity-controls section p {
  margin: 0;
}

.backend-parity-rule {
  display: grid;
  grid-template-columns: 1fr auto 1fr auto 1fr;
  align-items: center;
  gap: 5px;
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.055);
  padding: 10px;
  font-size: 0.68rem;
  text-align: center;
}

.backend-parity-rule strong {
  color: #1d6849;
}

.backend-parity-warning {
  border-radius: 9px;
  background: rgba(183, 121, 31, 0.07);
  padding: 11px;
}

@media (max-width: 1180px) {
  .workspace--backend-parity {
    grid-template-columns: 1fr;
  }

  .backend-parity-controls {
    position: static;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .backend-parity-controls > p,
  .backend-parity-controls > h2,
  .backend-parity-controls > .backend-parity-rule {
    grid-column: 1 / -1;
  }
}

@media (max-width: 820px) {
  .backend-parity-intro,
  .backend-parity-probe,
  .backend-parity-paper .panel-heading,
  .backend-parity-lanes .panel-heading,
  .backend-parity-inspector .panel-heading,
  .backend-parity-results .panel-heading {
    display: grid;
  }

  .backend-parity-chip {
    justify-self: start;
  }

  .backend-parity-lane-grid,
  .backend-parity-detail-grid,
  .backend-parity-controls {
    grid-template-columns: 1fr;
  }

  .backend-parity-controls > p,
  .backend-parity-controls > h2,
  .backend-parity-controls > .backend-parity-rule {
    grid-column: auto;
  }

  .backend-parity-paper-table,
  .backend-parity-results-table {
    overflow-x: auto;
  }

  .backend-parity-paper-table > div {
    min-width: 390px;
  }

  .backend-parity-results-table > div {
    min-width: 570px;
  }

  .backend-parity-probe-status {
    min-width: 0;
  }
}

.workspace--dynamic-autograd {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.autograd-stage {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.autograd-intro,
.autograd-graph-panel,
.autograd-saved-panel,
.autograd-backward-panel,
.autograd-audit-panel,
.autograd-controls {
  border: 1px solid $line;
  border-radius: 12px;
  background: $paper;
  box-shadow: 0 14px 35px rgba(23, 32, 28, 0.08);
}

.autograd-intro,
.autograd-graph-panel,
.autograd-saved-panel,
.autograd-backward-panel,
.autograd-audit-panel {
  padding: 16px;
}

.autograd-intro {
  display: flex;
  justify-content: space-between;
  gap: 18px;
  background:
    radial-gradient(circle at 88% 18%, rgba(109, 91, 208, 0.14), transparent 34%),
    $paper;
}

.autograd-intro h2,
.autograd-intro p,
.autograd-graph-panel h2,
.autograd-saved-panel h2,
.autograd-backward-panel h2,
.autograd-audit-panel h2,
.autograd-controls h2,
.autograd-mental-model h2,
.autograd-mental-model p {
  margin: 0;
}

.autograd-intro p:not(.eyebrow),
.autograd-controls > p,
.autograd-mental-model p {
  color: $muted;
  line-height: 1.55;
}

.autograd-chip {
  align-self: flex-start;
  flex: 0 0 auto;
  border: 1px solid rgba(109, 91, 208, 0.36);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.09);
  color: #4b3e9d;
  padding: 8px 12px;
  font-size: 0.72rem;
  font-weight: 850;
  text-transform: uppercase;
}

.autograd-graph-panel,
.autograd-saved-panel,
.autograd-backward-panel,
.autograd-audit-panel {
  display: grid;
  gap: 13px;
}

.autograd-order-strip {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.06);
  padding: 10px 12px;
}

.autograd-order-strip small,
.autograd-selected-grid small,
.autograd-mutation-strip small,
.autograd-backward-equations small,
.autograd-audit-grid small {
  color: $muted;
  font-size: 0.66rem;
  font-weight: 800;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.autograd-order-strip code,
.autograd-selected-grid code,
.autograd-mutation-strip code,
.autograd-backward-buttons code,
.autograd-backward-equations code,
.autograd-audit-grid code,
.autograd-branch-note code,
.autograd-scenario-buttons code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.autograd-order-strip code {
  min-width: 0;
  overflow-wrap: anywhere;
  color: #234c9f;
}

.autograd-node-lane {
  display: grid;
  grid-template-columns: repeat(6, minmax(105px, 1fr));
  gap: 7px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.autograd-node-lane button,
.autograd-backward-buttons button,
.autograd-scenario-buttons button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.82);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.autograd-node-lane button[aria-pressed="true"],
.autograd-backward-buttons button[aria-pressed="true"],
.autograd-scenario-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.09);
  box-shadow: inset 0 -3px rgba(109, 91, 208, 0.11);
}

.autograd-node-lane small,
.autograd-node-lane span,
.autograd-backward-buttons small,
.autograd-scenario-buttons span {
  color: $muted;
  font-size: 0.67rem;
}

.autograd-node-lane strong,
.autograd-backward-buttons strong,
.autograd-audit-grid strong,
.autograd-mutation-strip strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.autograd-branch-note {
  border-left: 3px solid rgba(183, 121, 31, 0.55);
  border-radius: 0 8px 8px 0;
  background: rgba(183, 121, 31, 0.07);
  color: #70501d;
  padding: 9px 11px;
  font-size: 0.78rem;
}

.autograd-selected-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 9px;
}

.autograd-selected-grid > div {
  display: grid;
  align-content: start;
  gap: 7px;
  min-width: 0;
  border: 1px solid rgba(37, 99, 235, 0.16);
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.045);
  padding: 12px;
}

.autograd-selected-grid code {
  color: #234c9f;
  overflow-wrap: anywhere;
}

.autograd-selected-grid strong {
  color: #1d6849;
}

.autograd-mutation-strip {
  display: grid;
  grid-template-columns: repeat(2, minmax(100px, 0.35fr)) minmax(180px, 1fr);
  gap: 8px;
}

.autograd-mutation-strip > div {
  display: grid;
  gap: 4px;
  border: 1px solid $line;
  border-radius: 8px;
  padding: 9px;
}

.autograd-mutation-strip > div.is-mutated {
  border-color: rgba(194, 65, 59, 0.4);
  background: rgba(194, 65, 59, 0.06);
}

.autograd-mutation-strip p {
  align-self: center;
  margin: 0;
  color: $muted;
  line-height: 1.45;
}

.autograd-backward-buttons {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.autograd-backward-buttons code {
  color: #9b3131;
  font-size: 0.7rem;
}

.autograd-backward-equations {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.autograd-backward-equations > div {
  display: grid;
  gap: 6px;
  min-width: 0;
  border: 1px solid rgba(194, 65, 59, 0.18);
  border-radius: 9px;
  background: rgba(194, 65, 59, 0.045);
  padding: 11px;
}

.autograd-backward-equations code {
  color: #9b3131;
  overflow-wrap: anywhere;
}

.autograd-backward-equations span {
  color: $muted;
  font-size: 0.7rem;
}

.autograd-audit-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.autograd-audit-grid > div {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 8px;
  padding: 10px;
}

.autograd-audit-grid span {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  color: $muted;
  font-size: 0.71rem;
}

.autograd-audit-grid code {
  color: #1d6849;
}

.autograd-audit-grid .autograd-audit-max {
  border-color: rgba(35, 122, 87, 0.28);
  background: rgba(35, 122, 87, 0.055);
}

.autograd-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 10px;
  padding: 16px;
}

.autograd-scenario-buttons {
  display: grid;
  gap: 7px;
}

.autograd-scenario-buttons code {
  color: #4b3e9d;
  font-size: 0.68rem;
  overflow-wrap: anywhere;
}

.autograd-mutation-toggle {
  border: 1px solid rgba(194, 65, 59, 0.35);
  border-radius: 9px;
  background: rgba(194, 65, 59, 0.06);
  color: #9b3131;
  padding: 10px;
  font: inherit;
  font-weight: 750;
}

.autograd-mental-model {
  display: grid;
  gap: 7px;
  margin-top: 4px;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.065);
  padding: 11px;
}

@media (max-width: 1180px) {
  .workspace--dynamic-autograd {
    grid-template-columns: 1fr;
  }

  .autograd-controls {
    position: static;
  }

  .autograd-scenario-buttons {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .autograd-intro,
  .autograd-graph-panel .panel-heading,
  .autograd-saved-panel .panel-heading,
  .autograd-backward-panel .panel-heading,
  .autograd-audit-panel .panel-heading {
    display: grid;
  }

  .autograd-chip {
    justify-self: start;
  }

  .autograd-order-strip,
  .autograd-selected-grid,
  .autograd-mutation-strip,
  .autograd-backward-buttons,
  .autograd-backward-equations,
  .autograd-audit-grid,
  .autograd-scenario-buttons {
    grid-template-columns: 1fr;
  }

  .autograd-node-lane {
    grid-template-columns: repeat(6, minmax(118px, 1fr));
  }
}

.workspace--tensor-broadcasting {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.tensor-broadcast-stage {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.tensor-broadcast-intro,
.tensor-shape-panel,
.tensor-output-panel,
.tensor-mapping-panel,
.tensor-gradient-panel,
.tensor-mismatch-panel {
  border: 1px solid $line;
  border-radius: 12px;
  background: $paper;
  box-shadow: 0 14px 35px rgba(23, 32, 28, 0.08);
  padding: 16px;
}

.tensor-broadcast-intro {
  display: flex;
  justify-content: space-between;
  gap: 18px;
  background:
    radial-gradient(circle at 88% 18%, rgba(37, 99, 235, 0.12), transparent 32%),
    $paper;
}

.tensor-broadcast-intro h2,
.tensor-broadcast-intro p,
.tensor-shape-panel h2,
.tensor-output-panel h2,
.tensor-mapping-panel h2,
.tensor-gradient-panel h2,
.tensor-mismatch-panel h2,
.tensor-mismatch-panel p {
  margin: 0;
}

.tensor-broadcast-intro > div:first-child {
  display: grid;
  gap: 7px;
  max-width: 760px;
}

.tensor-broadcast-chip {
  align-self: start;
  border: 1px solid rgba(37, 99, 235, 0.25);
  border-radius: 999px;
  background: rgba(37, 99, 235, 0.08);
  color: #234c9f;
  padding: 7px 11px;
  font-size: 0.72rem;
  font-weight: 800;
  text-transform: uppercase;
}

.tensor-shape-panel,
.tensor-output-panel,
.tensor-mapping-panel,
.tensor-gradient-panel {
  display: grid;
  gap: 12px;
}

.tensor-shape-equation {
  display: grid;
  grid-template-columns: minmax(100px, 1fr) auto minmax(100px, 1fr) auto minmax(100px, 1fr);
  align-items: center;
  gap: 8px;
}

.tensor-shape-equation code,
.tensor-shape-equation strong {
  min-width: 0;
  border-radius: 9px;
  padding: 12px;
  text-align: center;
  overflow-wrap: anywhere;
}

.tensor-shape-equation code {
  background: rgba(37, 99, 235, 0.065);
  color: #234c9f;
}

.tensor-shape-equation strong {
  background: rgba(35, 122, 87, 0.09);
  color: #1d6849;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.tensor-shape-equation span {
  color: $muted;
  font-weight: 900;
}

.tensor-axis-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.tensor-axis-grid > div {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 5px 8px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.7);
  padding: 10px;
}

.tensor-axis-grid > div.is-mismatch {
  border-color: rgba(194, 65, 59, 0.45);
  background: rgba(194, 65, 59, 0.07);
}

.tensor-axis-grid small,
.tensor-axis-grid span {
  color: $muted;
}

.tensor-axis-grid small {
  font-size: 0.68rem;
  font-weight: 800;
  text-transform: uppercase;
}

.tensor-axis-grid code {
  color: #4b3e9d;
  font-weight: 800;
}

.tensor-axis-grid strong {
  color: #1d6849;
}

.tensor-axis-grid span {
  grid-column: 1 / -1;
  font-size: 0.7rem;
}

.tensor-output-grid {
  display: grid;
  grid-template-columns: repeat(var(--tensor-columns), minmax(70px, 1fr));
  gap: 8px;
}

.tensor-output-grid button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.045);
  color: $ink;
  padding: 12px 8px;
}

.tensor-output-grid button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.52);
  background: rgba(109, 91, 208, 0.11);
  box-shadow: inset 0 0 0 1px rgba(109, 91, 208, 0.12);
}

.tensor-output-grid small {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.68rem;
}

.tensor-output-grid strong {
  color: #234c9f;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.05rem;
}

.tensor-mapping-equation {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: stretch;
  gap: 8px;
}

.tensor-mapping-equation > div {
  display: grid;
  gap: 6px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  padding: 11px;
}

.tensor-mapping-equation > strong {
  align-self: center;
  color: $muted;
}

.tensor-mapping-equation small,
.tensor-gradient-grid small,
.tensor-gradient-audit small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  text-transform: uppercase;
}

.tensor-mapping-equation code,
.tensor-gradient-grid code,
.tensor-gradient-audit code,
.tensor-mismatch-panel code,
.tensor-scenario-buttons code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.tensor-mapping-equation code {
  color: #4b3e9d;
  overflow-wrap: anywhere;
}

.tensor-mapping-equation span {
  color: $muted;
  font-size: 0.7rem;
}

.tensor-gradient-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.tensor-gradient-grid > div,
.tensor-gradient-audit > div {
  display: grid;
  gap: 6px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  padding: 10px;
}

.tensor-gradient-grid code,
.tensor-gradient-audit code {
  color: #1d6849;
  overflow-wrap: anywhere;
}

.tensor-gradient-grid span {
  color: $muted;
  font-size: 0.7rem;
}

.tensor-gradient-audit {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.tensor-mismatch-panel {
  display: grid;
  gap: 10px;
  border-color: rgba(194, 65, 59, 0.34);
  background: rgba(194, 65, 59, 0.055);
}

.tensor-mismatch-panel code {
  border-radius: 9px;
  background: rgba(194, 65, 59, 0.09);
  color: #9c332e;
  padding: 12px;
  overflow-wrap: anywhere;
}

.tensor-broadcast-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 10px;
  padding: 16px;
}

.tensor-scenario-buttons {
  display: grid;
  gap: 7px;
}

.tensor-scenario-buttons button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.tensor-scenario-buttons button[aria-pressed="true"] {
  border-color: rgba(37, 99, 235, 0.48);
  background: rgba(37, 99, 235, 0.085);
}

.tensor-scenario-buttons code {
  color: #234c9f;
  font-size: 0.72rem;
}

.tensor-scenario-buttons span {
  color: $muted;
  font-size: 0.7rem;
}

.tensor-mental-model {
  display: grid;
  gap: 7px;
  margin-top: 6px;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.07);
  padding: 11px;
}

.tensor-mental-model h2,
.tensor-mental-model p {
  margin: 0;
}

@media (max-width: 1180px) {
  .workspace--tensor-broadcasting {
    grid-template-columns: 1fr;
  }

  .tensor-broadcast-controls {
    position: static;
  }

  .tensor-scenario-buttons {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .tensor-broadcast-intro,
  .tensor-shape-panel .panel-heading,
  .tensor-output-panel .panel-heading,
  .tensor-mapping-panel .panel-heading,
  .tensor-gradient-panel .panel-heading {
    display: grid;
  }

  .tensor-broadcast-chip {
    justify-self: start;
  }

  .tensor-shape-equation,
  .tensor-mapping-equation,
  .tensor-gradient-grid,
  .tensor-gradient-audit,
  .tensor-scenario-buttons {
    grid-template-columns: 1fr;
  }

  .tensor-axis-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .tensor-output-grid {
    grid-template-columns: repeat(var(--tensor-columns), minmax(58px, 1fr));
  }
}

.structured-workbench {
  display: grid;
  gap: 14px;
}

.structured-lab-switch {
  display: flex;
  gap: 8px;
  padding: 0 24px;
}

.structured-lab-switch button {
  border: 1px solid $line;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.72);
  color: $ink;
  padding: 10px 16px;
  font: inherit;
  font-weight: 700;
}

.structured-lab-switch button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.38);
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
}

.workspace--hopfield {
  grid-template-columns: minmax(0, 1fr) 286px;
  align-items: start;
  gap: 18px;
}

.hopfield-stage {
  min-width: 0;
  display: grid;
  gap: 16px;
}

.hopfield-intro,
.hopfield-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.hopfield-intro h2,
.hopfield-heading h2,
.hopfield-controls h2 {
  margin: 0;
}

.hopfield-intro > div:first-child > p:last-child,
.hopfield-heading p,
.hopfield-controls > p {
  color: $muted;
  line-height: 1.55;
}

.hopfield-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(35, 122, 87, 0.25);
  border-radius: 999px;
  background: rgba(35, 122, 87, 0.08);
  color: #1d6849;
  padding: 8px 12px;
  font-size: 0.78rem;
  font-weight: 800;
}

.hopfield-store-panel,
.hopfield-recall-panel,
.hopfield-controls {
  border: 1px solid $line;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.78);
  box-shadow: 0 18px 50px rgba(29, 45, 39, 0.08);
}

.hopfield-store-panel,
.hopfield-recall-panel {
  padding: 18px;
}

.hopfield-heading > code {
  flex: 0 0 auto;
  border-radius: 8px;
  background: rgba(38, 99, 235, 0.08);
  color: #234c9f;
  padding: 8px 10px;
  font-size: 0.78rem;
}

.hopfield-pattern-row {
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) auto minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  margin: 16px 0;
}

.hopfield-pattern-row > div,
.hopfield-state,
.hopfield-update,
.hopfield-audit-grid > div,
.hopfield-contribution-panel > div {
  display: grid;
  gap: 5px;
  border: 1px solid rgba(45, 55, 72, 0.12);
  border-radius: 10px;
  background: rgba(248, 249, 246, 0.86);
  padding: 11px;
}

.hopfield-pattern-row small,
.hopfield-state small,
.hopfield-update small,
.hopfield-audit-grid small,
.hopfield-contribution-panel small,
.hopfield-selected-summary small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.hopfield-pattern-row strong,
.hopfield-state strong,
.hopfield-update strong,
.hopfield-audit-grid strong,
.hopfield-contribution-panel strong,
.hopfield-selected-summary strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.hopfield-matrix {
  display: grid;
  grid-template-columns: 72px repeat(4, minmax(70px, 1fr));
  gap: 5px;
  align-items: center;
}

.hopfield-matrix > b {
  color: $muted;
  font-size: 0.7rem;
  text-align: center;
}

.hopfield-matrix__row {
  display: contents;
}

.hopfield-matrix__row > b {
  color: $muted;
  font-size: 0.7rem;
}

.hopfield-weight {
  border: 1px solid rgba(38, 99, 235, 0.16);
  border-radius: 8px;
  background: rgba(38, 99, 235, 0.08);
  padding: 9px 6px;
  text-align: center;
}

.hopfield-weight--diagonal {
  border-color: rgba(183, 121, 31, 0.2);
  background: rgba(183, 121, 31, 0.09);
  color: #8a5a17;
}

.hopfield-note {
  margin: 14px 0 0;
  color: $muted;
  font-size: 0.82rem;
  line-height: 1.55;
}

.hopfield-recall-lane {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
  margin: 16px 0;
}

.hopfield-state {
  border-color: rgba(194, 65, 59, 0.24);
  background: rgba(194, 65, 59, 0.07);
}

.hopfield-update {
  opacity: 0.48;
}

.hopfield-update--visible {
  border-color: rgba(35, 122, 87, 0.28);
  background: rgba(35, 122, 87, 0.08);
  opacity: 1;
}

.hopfield-state span,
.hopfield-update span {
  color: $muted;
  font-size: 0.72rem;
}

.hopfield-audit-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr;
  gap: 8px;
  margin-bottom: 12px;
}

.hopfield-contribution-panel {
  display: grid;
  grid-template-columns: 0.65fr 2.3fr 1.2fr 1.2fr 1.2fr;
  gap: 8px;
  align-items: stretch;
}

.hopfield-contribution-panel > p {
  grid-column: 1 / -1;
  margin: 0;
  border-radius: 10px;
  background: rgba(109, 91, 208, 0.08);
  color: #50469a;
  padding: 12px;
  line-height: 1.5;
}

.hopfield-contributions {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.hopfield-contributions code {
  border-radius: 6px;
  background: rgba(38, 99, 235, 0.07);
  padding: 4px 6px;
  font-size: 0.7rem;
}

.hopfield-controls {
  position: sticky;
  top: 18px;
  padding: 16px;
}

.hopfield-controls > p:first-child {
  margin: 0 0 4px;
  color: #237a57;
  font-size: 0.7rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.hopfield-phase-buttons {
  display: grid;
  gap: 7px;
  margin-top: 14px;
}

.hopfield-phase-buttons button {
  display: grid;
  gap: 2px;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.8);
  color: $ink;
  padding: 9px 10px;
  text-align: left;
}

.hopfield-phase-buttons button span {
  color: $muted;
  font-size: 0.66rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.hopfield-phase-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.42);
  background: rgba(109, 91, 208, 0.11);
  color: #4b3e9d;
}

.hopfield-selected-summary {
  display: grid;
  gap: 5px;
  margin-top: 12px;
  border-radius: 10px;
  background: rgba(35, 122, 87, 0.08);
  padding: 12px;
}

.hopfield-selected-summary span,
.hopfield-selected-summary b {
  color: $muted;
  font-size: 0.75rem;
}

.hopfield-selected-summary b {
  color: #1d6849;
}

@media (max-width: 1180px) {
  .workspace--hopfield {
    grid-template-columns: 1fr;
  }

  .hopfield-controls {
    position: static;
  }

  .hopfield-phase-buttons {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .structured-lab-switch {
    padding: 0 12px;
  }

  .structured-lab-switch button {
    width: 100%;
  }

  .hopfield-intro,
  .hopfield-heading {
    display: grid;
  }

  .hopfield-chip,
  .hopfield-heading > code {
    justify-self: start;
  }

  .hopfield-pattern-row,
  .hopfield-recall-lane,
  .hopfield-audit-grid,
  .hopfield-contribution-panel,
  .hopfield-contributions,
  .hopfield-phase-buttons {
    grid-template-columns: 1fr;
  }

  .hopfield-pattern-row > span {
    justify-self: center;
  }

  .hopfield-matrix {
    grid-template-columns: 54px repeat(4, minmax(0, 1fr));
  }

  .hopfield-weight {
    padding: 8px 2px;
    font-size: 0.7rem;
  }
}

.workspace--message-passing {
  grid-template-columns: minmax(0, 1fr) 286px;
  align-items: start;
  gap: 18px;
}

.message-stage {
  min-width: 0;
  display: grid;
  gap: 16px;
}

.message-intro,
.message-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.message-intro h2,
.message-heading h2,
.message-controls h2 {
  margin: 0;
}

.message-intro > div:first-child > p:last-child,
.message-heading p,
.message-controls > p,
.message-sync-note {
  color: $muted;
  line-height: 1.55;
}

.message-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(37, 99, 235, 0.25);
  border-radius: 999px;
  background: rgba(37, 99, 235, 0.08);
  color: #234c9f;
  padding: 8px 12px;
  font-size: 0.78rem;
  font-weight: 800;
}

.message-graph-panel,
.message-update-panel,
.message-controls {
  border: 1px solid $line;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.78);
  box-shadow: 0 18px 50px rgba(29, 45, 39, 0.08);
}

.message-graph-panel,
.message-update-panel {
  padding: 18px;
}

.message-heading > code {
  flex: 0 0 auto;
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.08);
  color: #234c9f;
  padding: 8px 10px;
  font-size: 0.78rem;
}

.message-graph {
  display: grid;
  grid-template-columns: 1fr auto 1fr auto 1fr;
  align-items: center;
  gap: 10px;
  margin: 18px 0;
}

.message-node {
  display: grid;
  gap: 4px;
  min-height: 104px;
  border-radius: 50%;
  background: rgba(35, 122, 87, 0.07);
}

.message-node small,
.message-card small,
.message-inbox small,
.message-equation small,
.message-selected-summary small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.message-node strong,
.message-card strong,
.message-inbox strong,
.message-equation strong,
.message-selected-summary strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.05rem;
}

.message-node span {
  color: $muted;
  font-size: 0.68rem;
}

.message-node--selected {
  border-color: rgba(109, 91, 208, 0.55);
  background: rgba(109, 91, 208, 0.11);
  color: #4b3e9d;
}

.message-node:nth-child(1) {
  grid-column: 1;
}

.message-node:nth-child(2) {
  grid-column: 3;
}

.message-node:nth-child(3) {
  grid-column: 5;
}

.message-edge {
  color: #2563eb;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.75rem;
  font-weight: 800;
}

.message-edge--left {
  grid-column: 2;
  grid-row: 1;
}

.message-edge--right {
  grid-column: 4;
  grid-row: 1;
}

.message-ledger {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.message-card,
.message-inbox > div,
.message-equation > div {
  display: grid;
  gap: 5px;
  border: 1px solid rgba(45, 55, 72, 0.12);
  border-radius: 10px;
  background: rgba(248, 249, 246, 0.86);
  padding: 11px;
}

.message-card {
  opacity: 0.55;
}

.message-card--active {
  border-color: rgba(37, 99, 235, 0.34);
  background: rgba(37, 99, 235, 0.08);
  opacity: 1;
}

.message-card code,
.message-equation code {
  color: $muted;
  font-size: 0.72rem;
}

.message-inbox {
  display: grid;
  grid-template-columns: 2fr auto 1fr;
  align-items: center;
  gap: 10px;
  margin: 16px 0 10px;
}

.message-equation {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr) auto) minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: stretch;
  gap: 7px;
}

.message-equation > span,
.message-inbox > span {
  align-self: center;
  color: $muted;
  font-weight: 800;
}

.message-output {
  border-color: rgba(35, 122, 87, 0.28) !important;
  background: rgba(35, 122, 87, 0.08) !important;
}

.message-sync-note {
  margin: 12px 0 0;
  border-radius: 9px;
  background: rgba(183, 121, 31, 0.08);
  padding: 10px;
  font-size: 0.8rem;
}

.message-controls {
  position: sticky;
  top: 18px;
  padding: 16px;
}

.message-controls > p:first-child {
  margin: 0 0 4px;
  color: #237a57;
  font-size: 0.7rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.message-phase-buttons {
  display: grid;
  gap: 7px;
  margin-top: 14px;
}

.message-phase-buttons button {
  display: grid;
  gap: 2px;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.8);
  color: $ink;
  padding: 9px 10px;
  text-align: left;
}

.message-phase-buttons button span {
  color: $muted;
  font-size: 0.66rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.message-phase-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.42);
  background: rgba(109, 91, 208, 0.11);
  color: #4b3e9d;
}

.message-selected-summary {
  display: grid;
  gap: 5px;
  margin-top: 12px;
  border-radius: 10px;
  background: rgba(35, 122, 87, 0.08);
  padding: 12px;
}

.message-selected-summary span,
.message-selected-summary b {
  color: $muted;
  font-size: 0.75rem;
}

.message-selected-summary b {
  color: #1d6849;
}

@media (max-width: 1180px) {
  .workspace--message-passing {
    grid-template-columns: 1fr;
  }

  .message-controls {
    position: static;
  }

  .message-phase-buttons {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .message-intro,
  .message-heading {
    display: grid;
  }

  .message-chip,
  .message-heading > code {
    justify-self: start;
  }

  .message-graph,
  .message-ledger,
  .message-inbox,
  .message-equation,
  .message-phase-buttons {
    grid-template-columns: 1fr;
  }

  .message-node {
    grid-column: 1 !important;
    min-height: 76px;
    border-radius: 12px;
  }

  .message-edge,
  .message-equation > span,
  .message-inbox > span {
    justify-self: center;
  }

  .message-edge {
    grid-column: 1;
    grid-row: auto;
  }
}

.workspace--graph-neighborhood {
  grid-template-columns: minmax(0, 1fr) 286px;
  align-items: start;
  gap: 18px;
}

.graph-neighborhood-stage {
  min-width: 0;
  display: grid;
  gap: 16px;
}

.graph-neighborhood-intro,
.graph-neighborhood-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.graph-neighborhood-intro h2,
.graph-neighborhood-heading h2,
.graph-neighborhood-controls h2 {
  margin: 0;
}

.graph-neighborhood-intro p,
.graph-neighborhood-heading p,
.graph-neighborhood-map > p,
.graph-output-panel p,
.graph-neighborhood-controls > p {
  color: $muted;
  line-height: 1.55;
}

.graph-neighborhood-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(109, 91, 208, 0.3);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
  padding: 8px 12px;
  font-weight: 800;
}

.graph-neighborhood-map,
.graph-model-panel,
.graph-output-panel,
.graph-neighborhood-controls {
  border: 1px solid $line;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.78);
  box-shadow: 0 18px 50px rgba(29, 45, 39, 0.08);
}

.graph-neighborhood-map,
.graph-model-panel,
.graph-output-panel {
  padding: 18px;
}

.graph-neighborhood-heading > code {
  flex: 0 0 auto;
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.08);
  color: #234c9f;
  padding: 8px 10px;
  font-size: 0.76rem;
}

.graph-targets,
.graph-row-grid,
.graph-output-panel {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 9px;
}

.graph-targets {
  margin-top: 16px;
}

.graph-targets button,
.graph-row-grid > div,
.graph-output-panel > div {
  display: grid;
  gap: 5px;
  border: 1px solid rgba(45, 55, 72, 0.13);
  border-radius: 10px;
  background: rgba(248, 249, 246, 0.86);
  padding: 11px;
}

.graph-targets button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.11);
  color: #4b3e9d;
}

.graph-targets small,
.graph-row-grid small,
.graph-output-panel small,
.graph-neighborhood-controls small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.graph-targets strong,
.graph-row-grid strong,
.graph-output-panel strong,
.graph-result strong,
.graph-neighborhood-controls strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.graph-targets span,
.graph-row-grid span,
.graph-softmax-summary span,
.graph-neighborhood-controls span {
  color: $muted;
  font-size: 0.74rem;
}

.graph-row-grid {
  margin-top: 16px;
}

.graph-row-grid code {
  color: #234c9f;
  font-size: 0.72rem;
}

.graph-row-grid b {
  color: #237a57;
  font-size: 0.78rem;
}

.graph-result,
.graph-softmax-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 12px;
  border-radius: 10px;
  background: rgba(35, 122, 87, 0.08);
  padding: 12px;
}

.graph-result b {
  color: #1d6849;
}

.graph-output-panel {
  grid-template-columns: 1fr 1fr;
}

.graph-output-panel p {
  grid-column: 1 / -1;
  margin: 0;
}

.graph-neighborhood-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 9px;
  padding: 16px;
}

.graph-neighborhood-controls > p:first-child {
  margin: 0;
  color: #237a57;
  font-size: 0.7rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.graph-neighborhood-controls button,
.graph-neighborhood-controls > div {
  display: grid;
  gap: 3px;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.82);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.graph-neighborhood-controls button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.11);
  color: #4b3e9d;
}

.graph-neighborhood-controls > div {
  margin-top: 5px;
  background: rgba(35, 122, 87, 0.08);
}

@media (max-width: 1180px) {
  .workspace--graph-neighborhood {
    grid-template-columns: 1fr;
  }

  .graph-neighborhood-controls {
    position: static;
    grid-template-columns: 1fr 1fr;
  }

  .graph-neighborhood-controls > p,
  .graph-neighborhood-controls > h2,
  .graph-neighborhood-controls > div {
    grid-column: 1 / -1;
  }
}

@media (max-width: 820px) {
  .graph-neighborhood-intro,
  .graph-neighborhood-heading,
  .graph-result,
  .graph-softmax-summary {
    display: grid;
  }

  .graph-neighborhood-chip,
  .graph-neighborhood-heading > code {
    justify-self: start;
  }

  .graph-targets,
  .graph-row-grid,
  .graph-output-panel,
  .graph-neighborhood-controls {
    grid-template-columns: 1fr;
  }

  .graph-neighborhood-controls > p,
  .graph-neighborhood-controls > h2,
  .graph-neighborhood-controls > div {
    grid-column: auto;
  }
}

:root {
  color-scheme: light;
}

* {
  box-sizing: border-box;
}

html {
  font-size: 16px;
}

body {
  margin: 0;
  min-width: 320px;
  min-height: 100vh;
  background:
    linear-gradient(180deg, #fcfbf6 0%, $paper 72%),
    linear-gradient(90deg, rgba(35, 122, 87, 0.08), rgba(37, 99, 235, 0.05));
  color: $ink;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.5;
}

button,
input,
select {
  font: inherit;
}

button {
  min-height: 40px;
  border: 1px solid rgba(35, 122, 87, 0.28);
  border-radius: 7px;
  background: #ffffff;
  color: $ink;
  font-weight: 750;
  cursor: pointer;
}

button:hover {
  border-color: rgba(35, 122, 87, 0.64);
  background: #f3faf6;
}

input,
select {
  width: 100%;
  min-height: 38px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #ffffff;
  color: $ink;
  padding: 0 10px;
}

input[type="range"] {
  padding: 0;
  accent-color: $green;
}

#root {
  min-height: 100vh;
}

.app {
  width: min(1540px, calc(100vw - 28px));
  margin: 0 auto;
  padding: 22px 0 42px;
}

.app-header {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 16px;
}

.eyebrow {
  margin: 0 0 4px;
  color: $green;
  font-size: 0.76rem;
  font-weight: 850;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

h1,
h2 {
  margin: 0;
  line-height: 1.08;
  letter-spacing: 0;
}

h1 {
  font-size: clamp(1.7rem, 3vw, 2.7rem);
}

h2 {
  font-size: 1.1rem;
}

.formula {
  @include surface;
  min-width: 260px;
  padding: 13px 15px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.95rem;
  text-align: right;
}

.header-actions {
  display: grid;
  flex: 1;
  min-width: 0;
  align-items: start;
  gap: 10px;
}

.mode-toggle {
  display: grid;
  min-width: 0;
  grid-template-columns: repeat(9, minmax(0, 1fr));
  gap: 4px;
  padding: 4px;
  border: 1px solid $line;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.82);
}

.mode-button {
  min-width: 0;
  min-height: 34px;
  padding: 0 8px;
  border-color: transparent;
  white-space: nowrap;
}

.mode-button--active {
  border-color: rgba(37, 99, 235, 0.38);
  background: #edf4ff;
  color: #173f8a;
}

.workspace--lab {
  display: grid;
  grid-template-columns: 310px minmax(500px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.workspace--hidden {
  display: grid;
  grid-template-columns: 260px minmax(520px, 1fr) 310px;
  gap: 14px;
  align-items: start;
}

.workspace--microscope {
  display: grid;
  grid-template-columns: minmax(560px, 1fr) 320px;
  gap: 14px;
  align-items: start;
}

.workspace--optimization {
  display: grid;
  grid-template-columns: minmax(620px, 1fr) 320px;
  gap: 14px;
  align-items: start;
}

.microscope-stage {
  display: grid;
  gap: 12px;
}

.microscope-controls,
.microscope-focus,
.derivative-panel,
.before-after {
  @include surface;
}

.phase-strip {
  display: grid;
  grid-template-columns: repeat(7, minmax(76px, 1fr));
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.phase-button {
  display: grid;
  gap: 3px;
  width: 100%;
  min-height: 58px;
  padding: 7px 5px;
  border-color: $line;
  color: $muted;
  font-size: 0.72rem;
}

.phase-button span {
  display: block;
  width: 22px;
  height: 22px;
  margin: 0 auto;
  border-radius: 50%;
  background: #eef4f0;
  color: $ink;
  line-height: 22px;
}

.phase-button--complete {
  border-color: rgba(35, 122, 87, 0.42);
  color: $green;
}

.phase-button--complete span {
  background: rgba(35, 122, 87, 0.14);
  color: $green;
}

.phase-button--active {
  border-color: rgba(37, 99, 235, 0.58);
  background: #edf4ff;
  color: #173f8a;
}

.phase-button--active span {
  background: $blue;
  color: #ffffff;
}

.microscope-focus {
  display: grid;
  grid-template-columns: minmax(220px, 0.8fr) minmax(260px, 1fr);
  gap: 10px 24px;
  min-height: 180px;
  padding: 18px;
}

.microscope-focus code {
  align-self: center;
  min-height: 52px;
  padding: 14px;
  background: #edf4ff;
  color: #173f8a;
  font-size: 0.94rem;
}

.microscope-focus p {
  margin: 0;
  color: $muted;
}

.microscope-focus > p {
  grid-column: 1 / -1;
}

.focus-question {
  margin-top: 8px !important;
  font-weight: 750;
}

.signal-pipeline {
  display: grid;
  grid-template-columns: repeat(7, minmax(94px, 1fr));
  gap: 8px;
  overflow-x: auto;
  padding: 2px 0;
}

.signal-node {
  display: grid;
  align-content: center;
  gap: 7px;
  min-height: 92px;
  padding: 9px;
  border-color: rgba(35, 122, 87, 0.32);
  background: #ffffff;
}

.signal-node span {
  color: $muted;
  font-size: 0.68rem;
  text-transform: uppercase;
}

.signal-node strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.82rem;
}

.signal-node--active {
  border-color: rgba(37, 99, 235, 0.62);
  background: #edf4ff;
}

.signal-node--locked {
  border-style: dashed;
  background: rgba(255, 255, 255, 0.46);
  color: $muted;
}

.derivative-panel {
  display: grid;
  grid-template-columns: minmax(120px, 1fr) auto minmax(120px, 1fr) auto minmax(120px, 1fr) auto minmax(140px, 1fr);
  align-items: center;
  gap: 8px;
  padding: 14px;
}

.derivative-factor {
  display: grid;
  gap: 5px;
}

.derivative-factor span,
.before-after span {
  color: $muted;
  font-size: 0.7rem;
  font-weight: 850;
  text-transform: uppercase;
}

.derivative-factor--result code {
  background: #edf4ff;
  color: #173f8a;
}

.derivative-times,
.derivative-equals {
  color: $muted;
  font-weight: 850;
}

.before-after {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 14px;
  padding: 15px;
}

.before-after > div:not(.update-arrow) {
  display: grid;
  gap: 4px;
}

.before-after strong,
.before-after small {
  display: block;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.before-after small {
  color: $muted;
}

.update-arrow {
  color: $blue;
  font-size: 1.5rem;
  font-weight: 850;
}

.microscope-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.microscope-actions button {
  padding: 0 14px;
}

.microscope-actions button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.optimization-stage {
  display: grid;
  gap: 12px;
}

.landscape-panel,
.gradient-check-panel,
.batch-comparison-panel,
.optimization-controls {
  @include surface;
}

.landscape-panel,
.gradient-check-panel,
.batch-comparison-panel {
  padding: 14px;
}

.panel-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 10px;
}

.panel-heading > span {
  color: $muted;
  font-size: 0.78rem;
  font-weight: 800;
}

.landscape-chart {
  display: block;
  width: 100%;
  aspect-ratio: 720 / 430;
  border-radius: 8px;
  background: #ffffff;
}

.landscape-cell {
  fill: $blue;
}

.gradient-arrow {
  stroke: $red;
  stroke-width: 4;
}

.gradient-arrow-head {
  fill: $red;
}

.current-parameter-point {
  fill: $red;
  stroke: #ffffff;
  stroke-width: 3;
}

.optimum-point {
  fill: $green;
  stroke: #ffffff;
  stroke-width: 3;
}

.landscape-label {
  fill: $ink;
  font-size: 13px;
  font-weight: 850;
  paint-order: stroke;
  stroke: rgba(255, 255, 255, 0.86);
  stroke-width: 4px;
}

.axis-title--optimization-y {
  transform: rotate(-90deg);
  transform-origin: 18px 215px;
}

.landscape-equation {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-top: 10px;
}

.gradient-check-panel > p {
  margin: 10px 0 0;
  color: $muted;
}

.check-status {
  padding: 5px 9px;
  border-radius: 999px;
  letter-spacing: 0.06em;
}

.check-status--pass {
  background: rgba(35, 122, 87, 0.12);
  color: $green !important;
}

.check-status--fail {
  background: rgba(194, 65, 59, 0.12);
  color: $red !important;
}

.gradient-check-grid {
  display: grid;
  grid-template-columns: minmax(92px, 0.7fr) repeat(3, minmax(130px, 1fr));
  align-items: center;
  gap: 1px;
  overflow-x: auto;
}

.gradient-check-grid > * {
  min-width: 0;
  min-height: 38px;
  padding: 8px;
  border-bottom: 1px solid $line;
}

.gradient-check-grid > span {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 850;
  text-transform: uppercase;
}

.gradient-check-grid code {
  border-radius: 0;
  background: transparent;
}

.batch-chart {
  display: block;
  width: 100%;
  max-height: 210px;
  background: #ffffff;
}

.batch-grid {
  stroke: rgba(23, 32, 28, 0.16);
  stroke-width: 1;
}

.batch-line {
  fill: none;
  stroke-width: 3;
}

.batch-line--stochastic {
  stroke: $red;
}

.batch-line--mini-batch {
  stroke: $gold;
}

.batch-line--full-batch {
  stroke: $green;
}

.batch-axis-label {
  fill: $muted;
  font-size: 12px;
  font-weight: 800;
  text-anchor: middle;
}

.batch-axis-label--y {
  transform: rotate(-90deg);
  transform-origin: 12px 82px;
}

.strategy-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 9px;
  margin-top: 8px;
}

.strategy-summary {
  display: grid;
  gap: 4px;
  padding: 10px;
  border-left: 4px solid;
  background: rgba(255, 255, 255, 0.72);
}

.strategy-summary--stochastic {
  border-color: $red;
}

.strategy-summary--mini-batch {
  border-color: $gold;
}

.strategy-summary--full-batch {
  border-color: $green;
}

.strategy-summary span,
.strategy-summary small {
  color: $muted;
}

.strategy-summary code {
  margin-top: 4px;
}

.primary-action {
  border-color: $green;
  background: $green;
  color: #ffffff;
}

.primary-action:hover {
  border-color: #195b40;
  background: #195b40;
}

.lab-rail,
.chart-panel,
.metrics,
.lab-intro,
.trace-panel,
.network-panel {
  @include surface;
}

.lab-rail {
  max-height: calc(100vh - 126px);
  overflow: auto;
  padding: 14px;
}

.rail-summary {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 14px;
}

.rail-summary strong {
  font-size: 2rem;
  line-height: 1;
}

.rail-summary span {
  color: $muted;
  font-weight: 800;
}

.lab-group {
  display: grid;
  gap: 8px;
  margin-bottom: 16px;
}

.lab-group h2 {
  color: $muted;
  font-size: 0.76rem;
  text-transform: uppercase;
}

.lab-list {
  display: grid;
  gap: 6px;
}

.lab-button {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  padding: 7px 9px;
  border-color: $line;
  text-align: left;
}

.lab-button span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lab-button small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 850;
  text-transform: uppercase;
}

.lab-button--active {
  border-color: rgba(37, 99, 235, 0.52);
  background: #edf4ff;
}

.lab-stage {
  display: grid;
  gap: 12px;
}

.lab-intro {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 16px;
  padding: 15px;
}

.lab-intro p:not(.eyebrow) {
  margin: 7px 0 0;
  color: $muted;
}

.lab-chip {
  min-width: 86px;
  padding: 8px 10px;
  border-radius: 7px;
  background: #f4efe2;
  color: #67460d;
  font-size: 0.78rem;
  font-weight: 850;
  text-align: center;
}

.controls,
.metrics {
  display: grid;
  gap: 13px;
  padding: 15px;
}

.field,
.field-grid {
  display: grid;
  gap: 7px;
}

.field span,
.metric span,
.history__topline span,
.gradients span,
.lesson span,
.source-panel span {
  display: block;
  color: $muted;
  font-size: 0.72rem;
  font-weight: 850;
  text-transform: uppercase;
}

.field-grid {
  grid-template-columns: 1fr 1fr;
}

.button-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.chart-panel {
  padding: 12px;
}

.chart-panel svg {
  display: block;
  width: 100%;
  aspect-ratio: 720 / 460;
}

.chart-panel--hidden {
  min-height: 520px;
}

.chart-panel--hidden svg {
  max-height: 520px;
}

.plot-bg {
  fill: #ffffff;
}

.grid-line {
  stroke: rgba(23, 32, 28, 0.09);
  stroke-width: 1;
}

.axis-label,
.axis-title {
  fill: $muted;
  font-size: 13px;
  text-anchor: middle;
}

.axis-label--y {
  text-anchor: end;
}

.axis-title {
  font-weight: 850;
}

.axis-title--y {
  transform: rotate(-90deg);
  transform-origin: 20px 230px;
}

.ideal-line {
  fill: none;
  stroke: $gold;
  stroke-width: 2.5;
  stroke-dasharray: 8 7;
}

.model-line {
  fill: none;
  stroke: $blue;
  stroke-width: 4;
}

.hidden-curve {
  fill: none;
  stroke: $blue;
  stroke-width: 4;
}

.error-line {
  stroke: rgba(194, 65, 59, 0.42);
  stroke-width: 2;
}

.truth-point,
.prediction-point {
  stroke: #ffffff;
  stroke-width: 2;
}

.prediction-point {
  fill: $red;
}

.surface-chart {
  display: block;
  width: min(100%, 560px);
  aspect-ratio: 1;
  margin: 0 auto;
  border-radius: 8px;
  background: #ffffff;
}

.svg-button {
  cursor: pointer;
  outline: none;
}

.svg-button:focus .surface-point {
  stroke: $blue;
  stroke-width: 4;
}

.surface-point {
  stroke: #17201c;
  stroke-width: 2;
}

.surface-point--selected {
  stroke: $blue;
  stroke-width: 4;
}

.surface-label {
  fill: #17201c;
  font-size: 13px;
  font-weight: 850;
  paint-order: stroke;
  stroke: rgba(255, 255, 255, 0.78);
  stroke-width: 4px;
}

.hidden-table-chart {
  display: grid;
  gap: 8px;
  padding: 4px;
}

.table-row {
  display: grid;
  grid-template-columns: minmax(150px, 1fr) minmax(140px, 220px) 72px;
  align-items: center;
  gap: 10px;
  min-height: 48px;
  padding: 8px 10px;
  border-color: $line;
  text-align: left;
}

.table-row--selected {
  border-color: rgba(37, 99, 235, 0.5);
  background: #edf4ff;
}

.bar-pair {
  position: relative;
  display: block;
  height: 22px;
  border-radius: 6px;
  background: #eef4f0;
  overflow: hidden;
}

.bar-target,
.bar-prediction {
  position: absolute;
  left: 0;
  display: block;
  height: 10px;
}

.bar-target {
  top: 2px;
  background: rgba(35, 122, 87, 0.78);
}

.bar-prediction {
  bottom: 2px;
  background: rgba(194, 65, 59, 0.78);
}

.legend {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 18px;
  padding: 8px 4px 2px;
  color: $muted;
  font-size: 0.84rem;
  font-weight: 750;
}

.legend span {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.legend-dot--truth {
  background: $green;
}

.legend-dot--prediction {
  background: $red;
}

.legend-line {
  width: 22px;
  height: 0;
  border-top: 3px solid;
}

.legend-line--model {
  border-color: $blue;
}

.legend-line--ideal {
  border-color: $gold;
  border-top-style: dashed;
}

.metric {
  border-bottom: 1px solid $line;
  padding: 0 0 9px;
}

.metric strong {
  display: block;
  margin-top: 2px;
  font-size: 1.16rem;
}

.history {
  display: grid;
  gap: 8px;
  padding: 2px 0;
}

.history__topline {
  display: flex;
  justify-content: space-between;
  gap: 10px;
}

.history__topline strong {
  color: $muted;
  font-size: 0.78rem;
}

.history svg {
  width: 100%;
  height: 74px;
  overflow: visible;
}

.history-grid {
  stroke: rgba(23, 32, 28, 0.1);
  stroke-width: 1;
}

.history-line {
  fill: none;
  stroke: $green;
  stroke-width: 3;
}

.gradients,
.lesson,
.activation-panel,
.source-panel,
.trace-panel,
.network-panel {
  display: grid;
  gap: 6px;
}

.trace-panel {
  padding: 14px;
}

.network-panel {
  padding: 14px;
}

.network-svg {
  width: 100%;
  overflow: auto;
  border-radius: 8px;
  background: #ffffff;
}

.network-svg svg {
  display: block;
  width: auto;
  min-width: 100%;
  max-width: none;
  height: auto;
}

.lesson p,
.activation-panel p,
.source-panel p {
  margin: 0;
  color: $muted;
  font-size: 0.9rem;
}

.activation-panel svg {
  width: 100%;
  height: 82px;
  overflow: visible;
}

.activation-line {
  fill: none;
  stroke: $violet;
  stroke-width: 3;
}

code {
  display: block;
  min-height: 30px;
  padding: 6px 8px;
  border-radius: 6px;
  background: #eef4f0;
  color: #24352e;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.8rem;
}

.hidden-neuron-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(92px, 1fr));
  gap: 8px;
}

.neuron-tile {
  display: grid;
  gap: 4px;
  min-height: 76px;
  padding: 9px;
  border: 1px solid $line;
  border-radius: 8px;
  background: #ffffff;
}

.neuron-tile span {
  color: $muted;
  font-size: 0.74rem;
  font-weight: 850;
  text-transform: uppercase;
}

.neuron-tile strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.neuron-tile i {
  display: block;
  height: 8px;
  border-radius: 999px;
  background: $green;
}

.trace-equation code {
  margin-top: 4px;
}

.workspace--convolution {
  display: grid;
  grid-template-columns: minmax(620px, 1fr) 320px;
  gap: 14px;
  align-items: start;
}

.convolution-stage,
.convolution-controls,
.kernel-slide,
.mac-panel,
.output-strip,
.training-panel {
  @include surface;
}

.convolution-stage {
  display: grid;
  gap: 12px;
  padding: 14px;
}

.convolution-intro,
.mac-heading,
.array-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.convolution-intro p:not(.eyebrow),
.convolution-controls p,
.convolution-note p,
.training-heading p {
  margin: 5px 0 0;
  color: $muted;
}

.convolution-mode-chip {
  flex: 0 0 auto;
  padding: 6px 10px;
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.11);
  color: #4f3da9;
  font-size: 0.75rem;
  font-weight: 850;
}

.kernel-slide,
.mac-panel,
.output-strip {
  padding: 14px;
  overflow-x: auto;
}

.array-label {
  min-width: 420px;
  margin-bottom: 7px;
  color: $muted;
  font-size: 0.76rem;
  font-weight: 850;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.array-label code {
  min-height: 0;
  padding: 3px 7px;
  text-transform: none;
}

.array-label--kernel {
  margin-top: 14px;
}

.signal-array,
.kernel-track {
  display: grid;
  min-width: 420px;
  gap: 6px;
}

.signal-cell,
.kernel-cell {
  display: grid;
  place-items: center;
  min-height: 70px;
  border: 1px solid $line;
  border-radius: 8px;
  background: #ffffff;
  transition: background 120ms ease, border-color 120ms ease;
}

.signal-cell small,
.kernel-cell small,
.product-card small,
.accumulator-step small,
.output-button small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
}

.signal-cell strong,
.kernel-cell strong,
.product-card strong,
.accumulator-step strong,
.output-button strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.signal-cell--active {
  border-color: rgba(37, 99, 235, 0.58);
  background: #edf4ff;
}

.kernel-window {
  display: grid;
  gap: 6px;
  padding: 5px;
  border: 2px solid rgba(109, 91, 208, 0.55);
  border-radius: 10px;
  background: rgba(109, 91, 208, 0.07);
}

.kernel-cell {
  min-height: 58px;
  border-color: rgba(109, 91, 208, 0.26);
}

.mac-panel {
  display: grid;
  gap: 12px;
}

.mac-result {
  min-width: 72px;
  padding: 8px 12px;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.13);
  color: $green;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.35rem;
  text-align: center;
}

.product-grid,
.accumulator-strip,
.output-buttons {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(105px, 1fr));
  gap: 8px;
}

.product-card,
.accumulator-step {
  display: grid;
  gap: 5px;
  padding: 9px;
  border: 1px solid $line;
  border-radius: 8px;
  background: #ffffff;
}

.product-card code {
  min-height: 0;
  padding: 3px 6px;
}

.accumulator-step {
  border-color: rgba(35, 122, 87, 0.22);
  background: #f6fbf8;
}

.expanded-equation {
  min-height: 0;
  padding: 10px;
  overflow-x: auto;
  font-size: 0.9rem;
  white-space: nowrap;
}

.output-strip {
  display: grid;
  gap: 7px;
}

.output-button {
  display: grid;
  gap: 2px;
  min-height: 60px;
}

.output-button--active {
  border-color: rgba(35, 122, 87, 0.62);
  background: #eaf7ef;
  color: #17603f;
}

.training-panel {
  display: grid;
  gap: 14px;
  padding: 14px;
  overflow: hidden;
}

.training-heading,
.loss-flow {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.gradient-check-badge {
  display: grid;
  flex: 0 0 auto;
  min-width: 112px;
  padding: 8px 10px;
  border: 1px solid rgba(194, 65, 59, 0.3);
  border-radius: 8px;
  background: #fff5f4;
  color: $red;
  text-align: center;
}

.gradient-check-badge small,
.loss-flow small,
.kernel-update small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 850;
  text-transform: uppercase;
}

.gradient-check-badge--pass {
  border-color: rgba(35, 122, 87, 0.3);
  background: #eaf7ef;
  color: $green;
}

.loss-flow {
  justify-content: center;
  padding: 10px;
  border-block: 1px solid $line;
}

.loss-flow > div {
  display: grid;
  min-width: 140px;
  text-align: center;
}

.loss-flow strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.15rem;
}

.loss-flow span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.selected-gradient-path {
  display: grid;
  gap: 10px;
}

.selected-gradient-path h3 {
  margin: 0;
  font-size: 1rem;
}

.selected-gradient-path .mac-heading > code {
  min-height: 0;
  white-space: nowrap;
}

.gradient-table-wrap {
  overflow-x: auto;
  border-block: 1px solid $line;
}

.gradient-table {
  width: 100%;
  min-width: 650px;
  border-collapse: collapse;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
  text-align: right;
}

.gradient-table caption {
  padding: 10px 0 6px;
  color: $muted;
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  font-weight: 800;
  text-align: left;
}

.gradient-table th,
.gradient-table td {
  padding: 8px;
  border-bottom: 1px solid $line;
}

.gradient-table thead th,
.gradient-table tbody th {
  color: $muted;
  font-weight: 800;
}

.gradient-table thead th:first-child,
.gradient-table tbody th {
  text-align: left;
}

.gradient-sum {
  background: #f0f8f4;
  color: $green;
  font-weight: 850;
}

.kernel-update-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 8px;
}

.kernel-update {
  display: grid;
  gap: 5px;
  padding: 9px;
  border: 1px solid rgba(109, 91, 208, 0.22);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.06);
}

.kernel-update code {
  min-height: 0;
  padding: 3px 6px;
}

.kernel-update strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.convolution-controls {
  display: grid;
  gap: 14px;
  padding: 14px;
}

.convolution-training-controls {
  display: grid;
  gap: 10px;
  padding-block: 12px;
  border-block: 1px solid $line;
}

.training-step-button {
  border-color: rgba(109, 91, 208, 0.38);
  background: #f1efff;
  color: #4f3da9;
}

.convolution-controls .button-grid button:last-child {
  grid-column: 1 / -1;
}

.convolution-note {
  padding-top: 12px;
  border-top: 1px solid $line;
}

.convolution-note span {
  font-size: 0.78rem;
  font-weight: 850;
  text-transform: uppercase;
}

.convolution-error {
  padding: 20px;
  border: 1px solid rgba(194, 65, 59, 0.28);
  border-radius: 8px;
  background: #fff5f4;
  color: $red;
  font-weight: 750;
}

.workspace--image-cnn {
  display: grid;
  grid-template-columns: minmax(660px, 1fr) 320px;
  gap: 14px;
  align-items: start;
}

.image-cnn-stage,
.image-cnn-controls,
.image-stage-panel {
  @include surface;
}

.image-cnn-stage {
  display: grid;
  gap: 12px;
  min-width: 0;
  padding: 14px;
}

.image-cnn-intro,
.image-stage-heading,
.channel-math-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.image-cnn-intro p:not(.eyebrow),
.image-cnn-controls > p,
.image-cnn-note p {
  margin: 5px 0 0;
  color: $muted;
}

.image-shape-chip {
  flex: 0 0 auto;
  padding: 6px 10px;
  border-radius: 999px;
  background: rgba(37, 99, 235, 0.1);
  color: #173f8a;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.75rem;
  font-weight: 850;
}

.image-pipeline {
  display: grid;
  grid-template-columns: repeat(5, minmax(92px, 1fr));
  gap: 6px;
}

.image-stage-button {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 7px;
  min-height: 48px;
  padding: 6px 9px;
  color: $muted;
  text-align: left;
}

.image-stage-button small {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: rgba(23, 32, 28, 0.07);
  font-family: "SFMono-Regular", Consolas, monospace;
}

.image-stage-button--visited {
  border-color: rgba(35, 122, 87, 0.34);
  color: $green;
}

.image-stage-button--active {
  border-color: rgba(37, 99, 235, 0.52);
  background: #edf4ff;
  color: #173f8a;
}

.image-stage-button--active small {
  background: #173f8a;
  color: #ffffff;
}

.image-stage-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
  padding: 14px;
}

.image-stage-heading > code {
  min-height: 0;
  padding: 6px 9px;
  white-space: nowrap;
}

.image-channel-grid,
.channel-math-grid,
.image-map-pair,
.pooling-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.image-channel-card,
.channel-math-card,
.pooling-card {
  display: grid;
  gap: 10px;
  min-width: 0;
  padding: 11px;
  border: 1px solid $line;
  border-radius: 8px;
  background: #ffffff;
}

.image-channel-card > div:first-child,
.channel-math-title > div {
  display: grid;
}

.image-channel-card small,
.channel-math-card small,
.image-matrix-block > span,
.normalization-stats small,
.pooled-value small,
.image-control-group > span {
  color: $muted;
  font-size: 0.7rem;
  font-weight: 850;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.image-matrix-block {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.image-matrix {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.image-matrix-cell {
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 58px;
  padding: 5px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #f9faf7;
}

.image-matrix-cell small {
  color: $muted;
  font-size: 0.62rem;
}

.image-matrix-cell strong,
.channel-math-title > strong,
.normalization-stats strong,
.pooled-value strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.image-matrix-cell--selected {
  border-color: rgba(37, 99, 235, 0.62);
  background: #edf4ff;
  color: #173f8a;
}

.image-matrix-cell--winner {
  border-color: rgba(35, 122, 87, 0.62);
  background: #eaf7ef;
  color: #17603f;
}

.image-stage-note {
  margin: 0;
  padding: 10px;
  border-left: 3px solid rgba(35, 122, 87, 0.42);
  background: #f6fbf8;
  color: $muted;
}

.image-output-value {
  min-width: 68px;
  padding: 8px 12px;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.13);
  color: $green;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.35rem;
  text-align: center;
}

.window-kernel-pair {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 7px;
}

.window-kernel-pair > span,
.activation-flow > span,
.pooling-card > span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 850;
}

.channel-math-card .image-matrix-cell {
  min-height: 48px;
}

.image-product-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 5px;
}

.image-product-list code {
  min-height: 0;
  padding: 4px;
  font-size: 0.72rem;
  text-align: center;
}

.channel-reduction {
  display: grid;
  grid-template-columns: repeat(7, auto);
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 11px;
  border-block: 1px solid $line;
}

.channel-reduction > div {
  display: grid;
  min-width: 72px;
  text-align: center;
}

.channel-reduction small {
  color: $muted;
  font-size: 0.66rem;
  font-weight: 850;
  text-transform: uppercase;
}

.channel-reduction strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.channel-reduction__result {
  padding: 6px;
  border-radius: 7px;
  background: #eaf7ef;
  color: $green;
}

.normalization-flow {
  display: grid;
  grid-template-columns: minmax(160px, 1fr) minmax(170px, 0.8fr) minmax(160px, 1fr);
  align-items: center;
  gap: 12px;
}

.normalization-stats {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
}

.normalization-stats > div {
  display: grid;
  padding: 8px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #ffffff;
}

.normalization-equation {
  min-height: 0;
  padding: 10px;
  overflow-x: auto;
  text-align: center;
  white-space: nowrap;
}

.activation-flow {
  display: grid;
  grid-template-columns: minmax(180px, 1fr) auto minmax(180px, 1fr);
  align-items: center;
  gap: 12px;
}

.pooling-card {
  grid-template-columns: minmax(160px, 1fr) auto minmax(92px, 0.45fr);
  align-items: center;
}

.pooled-value {
  display: grid;
  place-items: center;
  min-height: 100px;
  padding: 8px;
  border: 1px solid rgba(35, 122, 87, 0.32);
  border-radius: 8px;
  background: #eaf7ef;
  color: $green;
  text-align: center;
}

.pooled-value strong {
  font-size: 1.35rem;
}

.pooled-value code {
  min-height: 0;
  padding: 2px 4px;
  font-size: 0.68rem;
}

.image-cnn-controls {
  display: grid;
  gap: 14px;
  padding: 14px;
}

.image-control-group {
  display: grid;
  gap: 7px;
  padding-block: 11px;
  border-top: 1px solid $line;
}

.image-filter-buttons,
.image-position-buttons {
  display: grid;
  gap: 7px;
}

.image-position-buttons {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.image-choice {
  display: grid;
  gap: 2px;
  min-width: 0;
  min-height: 54px;
  padding: 7px;
  text-align: left;
}

.image-choice small {
  color: $muted;
  font-size: 0.68rem;
}

.image-choice strong {
  overflow-wrap: anywhere;
}

.image-choice--active {
  border-color: rgba(37, 99, 235, 0.52);
  background: #edf4ff;
  color: #173f8a;
}

.image-stage-controls button:last-child {
  grid-column: 1 / -1;
}

.image-stage-controls button:disabled {
  cursor: default;
  opacity: 0.45;
}

.image-cnn-note {
  padding-top: 12px;
  border-top: 1px solid $line;
}

.image-cnn-note span {
  font-size: 0.78rem;
  font-weight: 850;
  text-transform: uppercase;
}

.workspace--residual {
  display: grid;
  grid-template-columns: minmax(660px, 1fr) 320px;
  gap: 14px;
  align-items: start;
}

.residual-stage,
.residual-controls,
.residual-block-panel,
.receptive-panel {
  @include surface;
}

.residual-stage {
  display: grid;
  gap: 12px;
  min-width: 0;
  padding: 14px;
}

.residual-intro,
.residual-panel-heading,
.residual-row-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.residual-intro p:not(.eyebrow),
.residual-controls > p,
.residual-note p {
  margin: 5px 0 0;
  color: $muted;
}

.residual-shape-chip {
  flex: 0 0 auto;
  padding: 6px 10px;
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.11);
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.75rem;
  font-weight: 850;
}

.residual-block-panel,
.receptive-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
  padding: 14px;
}

.residual-result {
  min-width: 70px;
  padding: 8px 12px;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.13);
  color: $green;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.35rem;
  text-align: center;
}

.residual-main-path {
  display: grid;
  gap: 8px;
  padding: 11px;
  border: 1px solid rgba(37, 99, 235, 0.2);
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.035);
}

.residual-lane-label,
.residual-row-label > span {
  color: $muted;
  font-size: 0.7rem;
  font-weight: 850;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.residual-signal-block {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.residual-row-label code {
  min-height: 0;
  padding: 3px 6px;
  font-size: 0.7rem;
}

.residual-signal-row {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.residual-cell {
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 58px;
  padding: 5px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #ffffff;
}

.residual-cell small {
  color: $muted;
  font-size: 0.64rem;
}

.residual-cell strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.residual-cell--active {
  border-color: rgba(109, 91, 208, 0.46);
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
}

.residual-cell--selected {
  border-color: rgba(37, 99, 235, 0.62);
  background: #edf4ff;
  color: #173f8a;
}

.residual-down-arrow {
  justify-self: center;
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.75rem;
}

.residual-skip-lane {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 14px;
  padding: 10px;
  border: 1px solid rgba(109, 91, 208, 0.3);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.07);
  color: #4f3da9;
}

.residual-skip-lane > div {
  display: grid;
}

.residual-skip-lane small,
.residual-addition small,
.field-width-badge small,
.hidden-path-card small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 850;
  text-transform: uppercase;
}

.residual-skip-lane code {
  min-height: 0;
  padding: 3px 7px;
}

.residual-skip-lane--disabled {
  filter: grayscale(1);
  opacity: 0.55;
}

.residual-addition {
  display: grid;
  grid-template-columns: repeat(7, auto);
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 11px;
  border-block: 1px solid $line;
}

.residual-addition > div {
  display: grid;
  min-width: 80px;
  text-align: center;
}

.residual-addition strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.residual-addition__output {
  padding: 6px;
  border-radius: 7px;
  background: #eaf7ef;
  color: $green;
}

.field-width-badge {
  display: grid;
  min-width: 100px;
  padding: 7px;
  border: 1px solid rgba(35, 122, 87, 0.3);
  border-radius: 8px;
  background: #eaf7ef;
  color: $green;
  text-align: center;
}

.field-width-badge strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.2rem;
}

.hidden-path-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: 8px;
}

.hidden-path-card {
  display: grid;
  gap: 6px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 8px;
  background: #ffffff;
}

.hidden-path-card > div {
  display: grid;
}

.hidden-path-card code {
  min-height: 0;
  padding: 4px 6px;
}

.hidden-path-card > span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
}

.path-count-table-wrap {
  overflow-x: auto;
  border-block: 1px solid $line;
}

.path-count-table {
  width: 100%;
  min-width: 600px;
  border-collapse: collapse;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
  text-align: right;
}

.path-count-table caption {
  padding: 9px 0 5px;
  color: $muted;
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  font-weight: 800;
  text-align: left;
}

.path-count-table th,
.path-count-table td {
  padding: 8px;
  border-bottom: 1px solid $line;
}

.path-count-table th:first-child {
  color: $muted;
  text-align: left;
}

.path-count-total {
  background: #eaf7ef;
  color: $green;
  font-weight: 850;
}

.receptive-summary {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 10px;
}

.receptive-summary code {
  min-height: 0;
  padding: 7px;
  white-space: nowrap;
}

.receptive-summary span {
  color: $muted;
}

.residual-controls {
  display: grid;
  gap: 14px;
  padding: 14px;
}

.residual-output-buttons {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 6px;
  padding-block: 11px;
  border-block: 1px solid $line;
}

.residual-output-button {
  display: grid;
  gap: 2px;
  min-width: 0;
  min-height: 58px;
  padding: 6px;
}

.residual-output-button small {
  color: $muted;
  font-size: 0.66rem;
}

.residual-output-button strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.residual-output-button--active {
  border-color: rgba(37, 99, 235, 0.52);
  background: #edf4ff;
  color: #173f8a;
}

.residual-skip-control {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  padding: 10px;
  border: 1px solid rgba(109, 91, 208, 0.3);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.07);
  cursor: pointer;
}

.residual-skip-control input {
  width: 20px;
  min-height: 20px;
  accent-color: $violet;
}

.residual-skip-control span {
  display: grid;
}

.residual-skip-control small {
  color: $muted;
}

.residual-controls .button-grid button:last-child {
  grid-column: 1 / -1;
}

.residual-controls button:disabled {
  cursor: default;
  opacity: 0.45;
}

.residual-note {
  padding-top: 12px;
  border-top: 1px solid $line;
}

.residual-note span {
  font-size: 0.78rem;
  font-weight: 850;
  text-transform: uppercase;
}

.workspace--recurrent {
  display: grid;
  grid-template-columns: minmax(760px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.recurrent-stage,
.recurrent-controls,
.recurrent-unroll-panel,
.memory-ablation-panel {
  @include surface;
}

.recurrent-stage {
  display: grid;
  gap: 12px;
  min-width: 0;
  padding: 14px;
}

.recurrent-intro,
.recurrent-panel-heading,
.recurrent-arithmetic-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.recurrent-intro p:not(.eyebrow),
.recurrent-controls > p,
.recurrent-panel-heading > p,
.recurrent-note p {
  margin: 5px 0 0;
  color: $muted;
}

.recurrent-sequence-chip {
  flex: 0 0 auto;
  padding: 6px 10px;
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.11);
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
  font-weight: 850;
}

.recurrent-unroll-panel,
.memory-ablation-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
  padding: 14px;
}

.recurrent-final-state {
  display: grid;
  min-width: 90px;
  padding: 8px 12px;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.13);
  color: $green;
  text-align: center;
}

.recurrent-final-state small,
.recurrent-initial-node small,
.recurrent-cell small,
.recurrent-equation small,
.recurrent-selected-summary small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 850;
  text-transform: uppercase;
}

.recurrent-final-state strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.35rem;
}

.shared-parameter-strip {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 9px;
  border: 1px solid rgba(109, 91, 208, 0.22);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.055);
}

.shared-parameter-strip span {
  color: #4f3da9;
  font-size: 0.7rem;
  font-weight: 850;
  text-transform: uppercase;
}

.shared-parameter-strip code {
  min-height: 0;
  padding: 4px 7px;
}

.recurrent-chain {
  display: grid;
  grid-template-columns: 94px 42px repeat(2, minmax(150px, 1fr) 42px) minmax(150px, 1fr);
  align-items: center;
  min-width: 0;
}

.recurrent-initial-node {
  display: grid;
  gap: 3px;
  place-items: center;
  min-height: 96px;
  padding: 9px;
  border: 1px solid $line;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.055);
}

.recurrent-initial-node code {
  min-height: 0;
  padding: 3px 8px;
}

.recurrent-connector {
  display: grid;
  place-items: center;
  color: #4f3da9;
}

.recurrent-connector small {
  color: $muted;
  font-size: 0.62rem;
  font-weight: 850;
  text-transform: uppercase;
}

.recurrent-connector span {
  font-size: 1.25rem;
}

.recurrent-connector--disabled {
  filter: grayscale(1);
  opacity: 0.42;
}

.recurrent-cell {
  display: grid;
  gap: 7px;
  min-width: 0;
  min-height: 112px;
  padding: 12px;
  border: 1px solid $line;
  border-radius: 8px;
  background: #ffffff;
  color: $ink;
  text-align: left;
}

.recurrent-cell:hover {
  border-color: rgba(37, 99, 235, 0.38);
  background: #f7faff;
}

.recurrent-cell span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.76rem;
}

.recurrent-cell strong {
  color: #173f8a;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1rem;
}

.recurrent-cell--active {
  border-color: rgba(37, 99, 235, 0.62);
  background: #edf4ff;
  box-shadow: inset 0 0 0 1px rgba(37, 99, 235, 0.14);
}

.recurrent-arithmetic {
  display: grid;
  gap: 12px;
  padding: 12px;
  border-block: 1px solid $line;
}

.recurrent-arithmetic-heading code {
  min-height: 0;
  padding: 4px 7px;
}

.recurrent-equation {
  display: grid;
  grid-template-columns: minmax(115px, 1fr) auto minmax(145px, 1.2fr) auto minmax(70px, 0.6fr) auto minmax(95px, 0.8fr) auto minmax(90px, 0.7fr);
  align-items: center;
  gap: 8px;
}

.recurrent-equation > div {
  display: grid;
  gap: 4px;
  min-width: 0;
  padding: 9px;
  border-radius: 7px;
  background: rgba(109, 91, 208, 0.055);
  text-align: center;
}

.recurrent-equation strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
}

.recurrent-equation > span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 850;
}

.recurrent-equation .equation-term--disabled {
  filter: grayscale(1);
  opacity: 0.5;
}

.recurrent-equation .recurrent-equation__state {
  background: #eaf7ef;
  color: $green;
}

.recurrent-table-wrap {
  min-width: 0;
  overflow-x: auto;
}

.recurrent-table {
  width: 100%;
  min-width: 580px;
  border-collapse: collapse;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
}

.recurrent-table caption {
  padding-bottom: 7px;
  color: $muted;
  font-family: inherit;
  font-size: 0.72rem;
  font-weight: 850;
  text-align: left;
}

.recurrent-table th,
.recurrent-table td {
  padding: 9px;
  border-bottom: 1px solid $line;
  text-align: right;
}

.recurrent-table th:first-child {
  text-align: left;
}

.recurrent-table thead th {
  color: $muted;
  font-size: 0.7rem;
  text-transform: uppercase;
}

.recurrent-table-row--active {
  background: #edf4ff;
  color: #173f8a;
}

.recurrent-controls {
  display: grid;
  gap: 12px;
  padding: 14px;
}

.recurrent-memory-control {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  padding: 11px;
  border: 1px solid rgba(109, 91, 208, 0.2);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.07);
  cursor: pointer;
}

.recurrent-memory-control input {
  width: 20px;
  min-height: 20px;
  accent-color: $violet;
}

.recurrent-memory-control span,
.recurrent-selected-summary {
  display: grid;
  gap: 3px;
}

.recurrent-memory-control small,
.recurrent-selected-summary span {
  color: $muted;
}

.recurrent-selected-summary {
  padding: 11px;
  border-left: 3px solid rgba(37, 99, 235, 0.45);
  background: rgba(37, 99, 235, 0.045);
}

.recurrent-selected-summary strong {
  color: #173f8a;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.recurrent-note {
  padding-top: 12px;
  border-top: 1px solid $line;
}

.recurrent-note span {
  font-size: 0.78rem;
  font-weight: 850;
  text-transform: uppercase;
}

.bptt-view-button {
  min-height: 42px;
  padding: 9px 12px;
  border: 1px solid rgba(109, 91, 208, 0.35);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.09);
  color: #4f3da9;
  font-weight: 850;
}

.bptt-view-button:hover {
  border-color: $violet;
  background: rgba(109, 91, 208, 0.15);
}

.workspace--bptt {
  display: grid;
  grid-template-columns: minmax(760px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.bptt-stage,
.bptt-panel {
  @include surface;
}

.bptt-stage {
  display: grid;
  gap: 12px;
  min-width: 0;
  padding: 14px;
}

.bptt-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
  padding: 14px;
}

.bptt-intro,
.bptt-panel-heading,
.bptt-arithmetic-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.bptt-intro p:not(.eyebrow),
.bptt-controls > p,
.bptt-update-panel > p {
  margin: 5px 0 0;
  color: $muted;
}

.bptt-loss-chip {
  display: grid;
  flex: 0 0 auto;
  min-width: 120px;
  padding: 9px 12px;
  border-radius: 8px;
  background: rgba(191, 64, 64, 0.1);
  color: #9b3131;
  text-align: center;
}

.bptt-loss-chip small,
.bptt-forward-lane small,
.bptt-step small,
.bptt-equation small,
.bptt-loss-change small {
  color: $muted;
  font-size: 0.67rem;
  font-weight: 850;
  text-transform: uppercase;
}

.bptt-loss-chip strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.25rem;
}

.bptt-panel-heading code {
  min-height: 0;
  padding: 4px 7px;
}

.bptt-forward-lane {
  display: grid;
  grid-template-columns: repeat(5, minmax(118px, 1fr));
  gap: 8px;
}

.bptt-forward-lane > div {
  display: grid;
  gap: 4px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 7px;
  background: rgba(35, 122, 87, 0.055);
}

.bptt-forward-lane strong,
.bptt-step strong,
.bptt-step span {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.bptt-forward-lane .bptt-forward-lane__loss {
  border-color: rgba(191, 64, 64, 0.2);
  background: rgba(191, 64, 64, 0.07);
}

.bptt-direction-label {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #4f3da9;
  font-size: 0.72rem;
  font-weight: 850;
  text-transform: uppercase;
}

.bptt-direction-label span {
  font-size: 1.2rem;
}

.bptt-backward-lane {
  display: grid;
  grid-template-columns: repeat(3, minmax(150px, 1fr));
  gap: 9px;
}

.bptt-step {
  display: grid;
  gap: 6px;
  min-width: 0;
  padding: 11px;
  border: 1px solid rgba(109, 91, 208, 0.22);
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.055);
  color: $ink;
  text-align: left;
}

.bptt-step:hover,
.bptt-step--active {
  border-color: rgba(109, 91, 208, 0.62);
  background: rgba(109, 91, 208, 0.13);
}

.bptt-step--active {
  box-shadow: inset 0 0 0 1px rgba(109, 91, 208, 0.12);
}

.bptt-step span {
  color: $muted;
  font-size: 0.75rem;
}

.bptt-arithmetic {
  display: grid;
  gap: 12px;
  padding-top: 13px;
  border-top: 1px solid $line;
}

.bptt-arithmetic-heading code {
  min-height: 0;
  padding: 4px 7px;
}

.bptt-equation {
  display: grid;
  grid-template-columns: repeat(2, minmax(85px, 1fr) auto) minmax(100px, 1fr);
  align-items: center;
  gap: 7px;
}

.bptt-equation > div {
  display: grid;
  gap: 4px;
  min-width: 0;
  padding: 9px;
  border-radius: 7px;
  background: rgba(37, 99, 235, 0.055);
  text-align: center;
}

.bptt-equation > span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 850;
}

.bptt-equation strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.bptt-equation .bptt-equation__result {
  background: rgba(109, 91, 208, 0.13);
  color: #4f3da9;
}

.bptt-local-gradients,
.bptt-parameter-update {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 7px;
}

.bptt-local-gradients code,
.bptt-parameter-update code {
  min-height: 0;
  padding: 6px 7px;
  white-space: normal;
}

.bptt-table-wrap {
  min-width: 0;
  overflow-x: auto;
}

.bptt-table {
  width: 100%;
  min-width: 570px;
  border-collapse: collapse;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
}

.bptt-table caption {
  padding-bottom: 7px;
  color: $muted;
  font-size: 0.72rem;
  font-weight: 850;
  text-align: left;
}

.bptt-table th,
.bptt-table td {
  padding: 9px;
  border-bottom: 1px solid $line;
  text-align: right;
}

.bptt-table th:first-child {
  text-align: left;
}

.bptt-table thead th {
  color: $muted;
  font-size: 0.69rem;
  text-transform: uppercase;
}

.bptt-table td:last-child {
  color: #4f3da9;
  background: rgba(109, 91, 208, 0.055);
}

.bptt-pass {
  color: $green;
  font-size: 0.72rem;
  letter-spacing: 0.06em;
}

.bptt-initial-gradient {
  margin: 0;
  padding: 8px 10px;
  border-left: 3px solid rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.04);
}

.bptt-audit-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(270px, 0.65fr);
  gap: 12px;
  min-width: 0;
}

.bptt-update-panel {
  align-content: start;
}

.bptt-loss-change {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 8px;
}

.bptt-loss-change > div {
  display: grid;
  gap: 4px;
  padding: 10px;
  border-radius: 7px;
  background: rgba(191, 64, 64, 0.07);
  text-align: center;
}

.bptt-loss-change > div:last-child {
  background: rgba(35, 122, 87, 0.1);
  color: $green;
}

.bptt-loss-change strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.05rem;
}

.bptt-parameter-update {
  grid-template-columns: 1fr;
}

.workspace--gates {
  display: grid;
  grid-template-columns: minmax(780px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.gate-stage,
.gate-controls,
.gate-comparison-panel {
  @include surface;
}

.gate-stage {
  display: grid;
  gap: 12px;
  min-width: 0;
  padding: 14px;
}

.gate-intro,
.gate-panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.gate-intro p:not(.eyebrow),
.gate-controls > p {
  margin: 5px 0 0;
  color: $muted;
}

.gate-input-chip {
  display: grid;
  flex: 0 0 auto;
  gap: 3px;
  min-width: 150px;
  padding: 9px 12px;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.1);
  color: $green;
  text-align: center;
}

.gate-input-chip small,
.gate-state-node small,
.gate-node small,
.gate-candidate-node small,
.gate-cell-node small,
.gate-result-node small,
.gate-selected-summary small {
  color: $muted;
  font-size: 0.67rem;
  font-weight: 850;
  text-transform: uppercase;
}

.gate-input-chip strong,
.gate-flow strong,
.gate-selected-summary strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.gate-comparison-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
  padding: 14px;
}

.gate-panel-heading code {
  min-height: 0;
  padding: 4px 7px;
}

.gate-model-lane {
  display: grid;
  grid-template-columns: 132px minmax(0, 1fr);
  align-items: stretch;
  gap: 10px;
  padding-block: 12px;
  border-block: 1px solid $line;
}

.gate-model-lane + .gate-model-lane {
  border-top: 0;
}

.gate-model-label {
  display: grid;
  align-content: center;
  gap: 5px;
  padding-right: 10px;
  border-right: 2px solid rgba(109, 91, 208, 0.25);
}

.gate-model-label span {
  color: #4f3da9;
  font-size: 1.2rem;
  font-weight: 900;
}

.gate-model-label strong {
  color: $muted;
  font-size: 0.72rem;
}

.gate-flow {
  display: grid;
  grid-template-columns: repeat(5, minmax(110px, 1fr));
  align-items: stretch;
  gap: 7px;
  min-width: 0;
}

.gate-flow--lstm {
  grid-template-columns: repeat(6, minmax(96px, 1fr));
}

.gate-state-node,
.gate-candidate-node,
.gate-cell-node,
.gate-result-node,
.gate-node {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  min-height: 96px;
  padding: 9px;
  border: 1px solid $line;
  border-radius: 8px;
  background: rgba(35, 122, 87, 0.04);
  color: $ink;
  text-align: left;
}

.gate-node {
  background: rgba(109, 91, 208, 0.055);
}

.gate-node:hover,
.gate-node--active {
  border-color: rgba(109, 91, 208, 0.62);
  background: rgba(109, 91, 208, 0.14);
}

.gate-node--active {
  box-shadow: inset 0 0 0 1px rgba(109, 91, 208, 0.12);
}

.gate-node span,
.gate-state-node span,
.gate-candidate-node span,
.gate-cell-node span,
.gate-result-node span {
  color: $muted;
  font-size: 0.7rem;
}

.gate-candidate-node {
  background: rgba(37, 99, 235, 0.06);
}

.gate-cell-node {
  background: rgba(205, 146, 38, 0.1);
}

.gate-result-node {
  background: rgba(35, 122, 87, 0.1);
  color: $green;
}

.gate-table-wrap {
  min-width: 0;
  overflow-x: auto;
}

.gate-table {
  width: 100%;
  min-width: 620px;
  border-collapse: collapse;
  font-size: 0.8rem;
}

.gate-table caption {
  padding-bottom: 7px;
  color: $muted;
  font-size: 0.72rem;
  font-weight: 850;
  text-align: left;
}

.gate-table th,
.gate-table td {
  padding: 9px;
  border-bottom: 1px solid $line;
  text-align: left;
}

.gate-table thead th {
  color: $muted;
  font-size: 0.7rem;
  text-transform: uppercase;
}

.gate-table tbody th {
  width: 30%;
}

.gate-controls {
  display: grid;
  gap: 12px;
  padding: 14px;
}

.gate-intervention-buttons {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}

.gate-intervention-buttons button {
  min-height: 39px;
  padding: 7px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #ffffff;
  color: $ink;
  font-size: 0.74rem;
  font-weight: 800;
}

.gate-intervention-buttons button:hover,
.gate-intervention-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.62);
  background: rgba(109, 91, 208, 0.13);
  color: #4f3da9;
}

.gate-selected-summary {
  display: grid;
  gap: 4px;
  padding: 11px;
  border-left: 3px solid rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.045);
}

.gate-selected-summary strong {
  color: #4f3da9;
  font-size: 1.35rem;
}

.gate-selected-summary span {
  color: $muted;
}

.workspace--attention {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.attention-stage,
.attention-controls {
  display: grid;
  gap: 14px;
}

.attention-intro,
.attention-panel-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.attention-intro {
  @include surface;
  padding: 18px;
}

.attention-intro p:last-child,
.attention-panel-heading p,
.attention-controls > p,
.attention-value-boundary p,
.attention-next-note p {
  margin: 7px 0 0;
  color: $muted;
}

.attention-sequence-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(109, 91, 208, 0.28);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.attention-projection-panel,
.attention-score-panel,
.attention-controls {
  @include surface;
  padding: 16px;
}

.attention-projection-table {
  display: grid;
  gap: 6px;
  margin-top: 14px;
}

.attention-projection-head,
.attention-projection-row {
  display: grid;
  grid-template-columns: 1.05fr repeat(3, 1fr);
  gap: 7px;
  align-items: stretch;
}

.attention-projection-head span {
  padding: 0 10px;
  color: $muted;
  font-size: 0.7rem;
  font-weight: 850;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.attention-projection-row > div {
  display: grid;
  align-content: center;
  gap: 2px;
  min-width: 0;
  min-height: 70px;
  padding: 9px 10px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #fbfcfa;
}

.attention-projection-row > div:first-child {
  grid-template-columns: auto 1fr;
  align-items: center;
}

.attention-projection-row > div:first-child code {
  grid-column: 2;
}

.attention-projection-row small,
.attention-cell-trace small,
.attention-selected-summary small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.attention-projection-row code {
  overflow-x: auto;
  color: #173f8a;
  white-space: nowrap;
}

.attention-token-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.attention-token-dot--red { background: $red; }
.attention-token-dot--blue { background: $blue; }
.attention-token-dot--purple { background: $violet; }

.attention-score-panel {
  display: grid;
  gap: 15px;
}

.attention-panel-heading > code {
  flex: 0 0 auto;
  padding: 6px 9px;
  border-radius: 6px;
  background: #edf4ff;
  color: #173f8a;
}

.attention-score-layout {
  display: grid;
  grid-template-columns: minmax(330px, 0.9fr) minmax(330px, 1.1fr);
  gap: 14px;
  align-items: stretch;
}

.attention-score-grid {
  display: grid;
  grid-template-columns: minmax(68px, 0.72fr) repeat(3, minmax(72px, 1fr));
  gap: 5px;
}

.attention-grid-label,
.attention-grid-corner {
  display: grid;
  place-items: center;
  min-height: 38px;
  color: $muted;
  font-size: 0.76rem;
  font-weight: 850;
}

.attention-grid-corner {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.attention-score-cell {
  min-width: 0;
  min-height: 70px;
  border-color: rgba(37, 99, 235, 0.2);
  background: rgba(37, 99, 235, 0.04);
  color: #173f8a;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.2rem;
}

.attention-score-cell:hover,
.attention-score-cell--active {
  border-color: rgba(109, 91, 208, 0.68);
  background: rgba(109, 91, 208, 0.13);
  color: #4f3da9;
}

.attention-score-cell--active {
  box-shadow: inset 0 0 0 2px rgba(109, 91, 208, 0.22);
}

.attention-cell-trace {
  display: grid;
  align-content: start;
  gap: 12px;
  padding: 14px;
  border-left: 4px solid rgba(109, 91, 208, 0.5);
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.045);
}

.attention-cell-trace h3 {
  margin: 0;
}

.attention-vector-pair {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.attention-vector-pair span {
  display: grid;
  gap: 2px;
  padding: 8px;
  border: 1px solid $line;
  border-radius: 6px;
  background: #ffffff;
}

.attention-dot-equation {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 12px;
  border-radius: 7px;
  background: #17201c;
  color: #ffffff;
}

.attention-dot-equation strong {
  color: #b9e7cf;
  font-size: 1.25rem;
}

.attention-products,
.attention-scale-equation {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.83rem;
}

.attention-scale-equation strong {
  color: #4f3da9;
}

.attention-controls {
  position: sticky;
  top: 14px;
}

.attention-scale-control {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 10px;
  align-items: start;
  padding: 11px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #fbfcfa;
}

.attention-scale-control input {
  width: auto;
  min-height: 0;
  margin-top: 4px;
  accent-color: $violet;
}

.attention-scale-control span,
.attention-selected-summary,
.attention-value-boundary,
.attention-next-note {
  display: grid;
  gap: 4px;
}

.attention-scale-control small,
.attention-selected-summary span {
  color: $muted;
}

.attention-selected-summary {
  padding: 12px;
  border-left: 3px solid rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.05);
}

.attention-selected-summary strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.7rem;
}

.attention-value-boundary,
.attention-next-note {
  padding-top: 12px;
  border-top: 1px solid $line;
}

.attention-value-boundary > span,
.attention-next-note > span {
  font-weight: 850;
}

.attention-value-boundary code {
  color: #173f8a;
}

.attention-back-button {
  border-color: rgba(109, 91, 208, 0.42);
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
}

.attention-back-button:hover {
  border-color: rgba(109, 91, 208, 0.68);
  background: rgba(109, 91, 208, 0.14);
}

.workspace--attention-softmax {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.attention-softmax-stage,
.attention-softmax-controls {
  display: grid;
  gap: 14px;
}

.attention-softmax-intro,
.attention-softmax-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.attention-softmax-intro,
.attention-weight-panel,
.attention-normalize-panel,
.attention-value-mix-panel,
.attention-softmax-controls {
  @include surface;
  padding: 16px;
}

.attention-softmax-intro p:last-child,
.attention-softmax-heading p,
.attention-softmax-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.attention-softmax-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(109, 91, 208, 0.28);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.attention-softmax-heading > code {
  flex: 0 0 auto;
  padding: 6px 9px;
  border-radius: 6px;
  background: #edf4ff;
  color: #173f8a;
}

.attention-weight-grid {
  display: grid;
  grid-template-columns: minmax(78px, 0.7fr) repeat(3, minmax(105px, 1fr));
  gap: 6px;
  margin-top: 14px;
}

.attention-weight-row-button {
  min-width: 0;
  min-height: 68px;
  border-color: transparent;
  background: transparent;
  color: $muted;
}

.attention-weight-row-button:hover,
.attention-weight-row-button--active {
  border-color: rgba(109, 91, 208, 0.4);
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
}

.attention-weight-cell {
  position: relative;
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 68px;
  overflow: hidden;
  border: 1px solid rgba(37, 99, 235, 0.17);
  border-radius: 7px;
  background: rgba(37, 99, 235, 0.035);
  color: #173f8a;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.attention-weight-cell span {
  position: absolute;
  inset: auto auto 0 0;
  height: 6px;
  background: rgba(37, 99, 235, 0.3);
}

.attention-weight-cell strong {
  position: relative;
  z-index: 1;
}

.attention-weight-cell--selected-row {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
}

.attention-weight-cell--selected-row span {
  background: rgba(109, 91, 208, 0.38);
}

.attention-weight-cell--blocked {
  border-color: rgba(194, 65, 59, 0.16);
  background:
    repeating-linear-gradient(
      -45deg,
      rgba(194, 65, 59, 0.025),
      rgba(194, 65, 59, 0.025) 7px,
      rgba(194, 65, 59, 0.075) 7px,
      rgba(194, 65, 59, 0.075) 8px
    );
  color: #8f342f;
  font-family: inherit;
  font-size: 0.76rem;
}

.attention-normalize-panel,
.attention-value-mix-panel {
  display: grid;
  gap: 14px;
}

.attention-normalize-flow {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr) auto) minmax(0, 1fr);
  gap: 8px;
  align-items: center;
}

.attention-normalize-flow > div {
  display: grid;
  gap: 4px;
  min-height: 76px;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #fbfcfa;
}

.attention-normalize-flow small,
.attention-context-result small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.attention-normalize-flow code {
  color: #173f8a;
  font-size: 0.8rem;
  overflow-wrap: anywhere;
  white-space: normal;
}

.attention-normalize-flow > span {
  color: $muted;
  font-weight: 850;
}

.attention-normalize-flow__result {
  border-color: rgba(109, 91, 208, 0.42) !important;
  background: rgba(109, 91, 208, 0.07) !important;
}

.attention-normalize-flow__result code {
  color: #4f3da9;
  font-weight: 850;
}

.attention-context-result {
  display: grid;
  gap: 2px;
  min-width: 150px;
  padding: 8px 10px;
  border-left: 3px solid rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.05);
}

.attention-context-result strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.25rem;
}

.attention-value-lanes {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.attention-value-lane {
  display: grid;
  gap: 7px;
  padding: 11px;
  border: 1px solid rgba(37, 99, 235, 0.19);
  border-radius: 7px;
  background: rgba(37, 99, 235, 0.035);
}

.attention-value-lane > span {
  display: flex;
  align-items: center;
  gap: 7px;
  font-weight: 800;
}

.attention-value-lane code {
  overflow-x: auto;
  color: #173f8a;
  white-space: nowrap;
}

.attention-value-lane--blocked {
  opacity: 0.58;
  border-color: $line;
  background: #fbfcfa;
}

.attention-softmax-controls {
  position: sticky;
  top: 14px;
}

.attention-query-buttons {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
}

.attention-query-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.62);
  background: rgba(109, 91, 208, 0.13);
  color: #4f3da9;
}

.workspace--multi-head {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.multi-head-stage,
.multi-head-controls {
  display: grid;
  gap: 14px;
}

.multi-head-intro,
.multi-head-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.multi-head-intro,
.multi-head-panel,
.multi-head-join-panel,
.multi-head-norm-panel,
.multi-head-controls {
  @include surface;
  padding: 16px;
}

.multi-head-intro p:last-child,
.multi-head-heading p,
.multi-head-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.multi-head-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(109, 91, 208, 0.28);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.08);
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.multi-head-panel,
.multi-head-join-panel,
.multi-head-norm-panel {
  display: grid;
  gap: 14px;
}

.multi-head-heading > code {
  flex: 0 0 auto;
  padding: 6px 9px;
  border-radius: 6px;
  background: #edf4ff;
  color: #173f8a;
}

.multi-head-lanes {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.multi-head-lane {
  display: grid;
  gap: 11px;
  min-width: 0;
  padding: 13px;
  border: 1px solid rgba(37, 99, 235, 0.2);
  border-top: 4px solid rgba(37, 99, 235, 0.46);
  border-radius: 7px;
  background: rgba(37, 99, 235, 0.035);
}

.multi-head-lane--vertical {
  border-color: rgba(109, 91, 208, 0.2);
  border-top-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.035);
}

.multi-head-lane__heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.multi-head-lane__heading > div {
  display: grid;
  gap: 3px;
}

.multi-head-lane__heading small,
.multi-head-score-row span,
.multi-head-join-flow small,
.multi-head-norm-flow small,
.multi-head-output small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.multi-head-lane__heading > code {
  color: #173f8a;
  font-weight: 800;
}

.multi-head-score-row {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  padding-top: 9px;
  border-top: 1px solid $line;
}

.multi-head-score-row code {
  overflow-wrap: anywhere;
  color: #173f8a;
}

.multi-head-weight-row {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
}

.multi-head-weight {
  position: relative;
  display: grid;
  gap: 4px;
  min-width: 0;
  min-height: 68px;
  overflow: hidden;
  padding: 8px;
  border: 1px solid rgba(37, 99, 235, 0.17);
  border-radius: 6px;
  background: #ffffff;
}

.multi-head-weight span {
  color: $muted;
  font-size: 0.72rem;
}

.multi-head-weight strong {
  position: relative;
  z-index: 1;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.multi-head-weight i {
  position: absolute;
  inset: auto auto 0 0;
  height: 6px;
  background: rgba(37, 99, 235, 0.34);
}

.multi-head-lane--vertical .multi-head-weight i {
  background: rgba(109, 91, 208, 0.38);
}

.multi-head-weight--blocked {
  opacity: 0.58;
  background:
    repeating-linear-gradient(
      -45deg,
      rgba(194, 65, 59, 0.025),
      rgba(194, 65, 59, 0.025) 7px,
      rgba(194, 65, 59, 0.075) 7px,
      rgba(194, 65, 59, 0.075) 8px
    );
}

.multi-head-value-row {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 5px;
}

.multi-head-value-row code {
  min-width: 0;
  overflow-wrap: anywhere;
  color: $muted;
  font-size: 0.74rem;
}

.multi-head-join-flow {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr) auto) minmax(0, 1fr);
  gap: 7px;
  align-items: center;
}

.multi-head-join-flow > div,
.multi-head-norm-flow > div {
  display: grid;
  gap: 5px;
  min-width: 0;
  min-height: 72px;
  align-content: center;
  padding: 9px;
  border: 1px solid $line;
  border-radius: 7px;
  background: #fbfcfa;
}

.multi-head-join-flow > span {
  color: $muted;
  font-weight: 850;
}

.multi-head-join-flow code,
.multi-head-norm-flow code {
  overflow-wrap: anywhere;
  color: #173f8a;
  font-size: 0.78rem;
  white-space: normal;
}

.multi-head-residual {
  border-color: rgba(20, 122, 83, 0.32) !important;
  background: rgba(20, 122, 83, 0.05) !important;
}

.multi-head-residual--off,
.multi-head-norm-flow--off {
  opacity: 0.5;
}

.multi-head-join-result,
.multi-head-norm-result {
  border-color: rgba(109, 91, 208, 0.42) !important;
  background: rgba(109, 91, 208, 0.07) !important;
}

.multi-head-join-result code,
.multi-head-norm-result code {
  color: #4f3da9;
  font-weight: 850;
}

.multi-head-norm-flow {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.multi-head-output {
  display: grid;
  gap: 2px;
  min-width: 170px;
  padding: 8px 10px;
  border-left: 3px solid rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.05);
}

.multi-head-output strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.18rem;
}

.multi-head-controls {
  position: sticky;
  top: 14px;
}

.workspace--decoder {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.decoder-stage,
.decoder-controls {
  display: grid;
  gap: 14px;
}

.decoder-intro,
.decoder-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.decoder-intro,
.decoder-shift-panel,
.decoder-prediction-panel,
.decoder-gradient-panel,
.decoder-update-panel,
.decoder-controls {
  @include surface;
  padding: 16px;
}

.decoder-intro p:last-child,
.decoder-heading p,
.decoder-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.decoder-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(35, 122, 87, 0.28);
  border-radius: 999px;
  background: rgba(35, 122, 87, 0.08);
  color: #185a40;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.decoder-shift-panel,
.decoder-prediction-panel,
.decoder-gradient-panel,
.decoder-update-panel {
  display: grid;
  gap: 14px;
}

.decoder-heading > code {
  flex: 0 0 auto;
  padding: 6px 9px;
  border-radius: 6px;
  background: #e9f7f0;
  color: #185a40;
}

.decoder-position-lanes {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.decoder-position-button {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  gap: 6px 10px;
  align-items: center;
  min-width: 0;
  padding: 11px;
  border: 1px solid rgba(35, 122, 87, 0.2);
  border-radius: 7px;
  background: rgba(35, 122, 87, 0.035);
  text-align: left;
}

.decoder-position-button:hover,
.decoder-position-button[aria-pressed="true"] {
  border-color: rgba(35, 122, 87, 0.55);
  background: rgba(35, 122, 87, 0.09);
}

.decoder-position-button span,
.decoder-position-button small {
  color: $muted;
  font-size: 0.72rem;
}

.decoder-position-button span,
.decoder-position-button small {
  grid-column: 1 / -1;
}

.decoder-position-button strong {
  overflow-wrap: anywhere;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.decoder-loss-badge {
  display: grid;
  gap: 2px;
  min-width: 150px;
  padding: 8px 10px;
  border-left: 3px solid rgba(194, 65, 59, 0.5);
  background: rgba(194, 65, 59, 0.05);
}

.decoder-loss-badge small,
.decoder-forward-flow small,
.decoder-softmax-trace small,
.decoder-gradient-flow small,
.decoder-update-grid small,
.decoder-loss-drop small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.decoder-loss-badge strong {
  color: #9f342f;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.18rem;
}

.decoder-forward-flow {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto minmax(0, 1fr) auto minmax(0, 1.2fr);
  align-items: stretch;
  gap: 7px;
}

.decoder-forward-flow > div,
.decoder-gradient-flow > div,
.decoder-softmax-trace > div,
.decoder-update-grid > div {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: $paper;
}

.decoder-forward-flow > span,
.decoder-gradient-flow > span {
  align-self: center;
  color: $muted;
  font-weight: 850;
}

.decoder-forward-flow code,
.decoder-gradient-flow code,
.decoder-softmax-trace code,
.decoder-update-grid code {
  overflow-wrap: anywhere;
  white-space: normal;
}

.decoder-target-node {
  border-color: rgba(35, 122, 87, 0.3) !important;
  background: rgba(35, 122, 87, 0.055) !important;
}

.decoder-vocabulary-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.decoder-vocabulary-row {
  position: relative;
  display: grid;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: #ffffff;
}

.decoder-vocabulary-row > div {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: space-between;
  gap: 8px;
}

.decoder-vocabulary-row > i {
  height: 6px;
  background: rgba(37, 99, 235, 0.35);
}

.decoder-vocabulary-row--target {
  border-color: rgba(35, 122, 87, 0.42);
  background: rgba(35, 122, 87, 0.045);
}

.decoder-vocabulary-row--target > i {
  background: rgba(35, 122, 87, 0.48);
}

.decoder-vocabulary-row code {
  overflow-wrap: anywhere;
  color: $muted;
  font-size: 0.72rem;
  white-space: normal;
}

.decoder-softmax-trace {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 7px;
}

.decoder-gradient-flow {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1.3fr) auto minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: stretch;
  gap: 7px;
}

.decoder-state-gradient {
  border-color: rgba(109, 91, 208, 0.28) !important;
  background: rgba(109, 91, 208, 0.05) !important;
}

.decoder-update-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 7px;
}

.decoder-loss-drop {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px;
  border-left: 4px solid rgba(35, 122, 87, 0.52);
  background: rgba(35, 122, 87, 0.055);
}

.decoder-gradient-audit {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 11px;
  border: 1px solid rgba(109, 91, 208, 0.24);
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.045);
}

.decoder-gradient-audit code {
  color: $muted;
}

.decoder-gradient-audit strong {
  margin-left: auto;
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.decoder-loss-drop > div {
  display: grid;
  gap: 2px;
}

.decoder-loss-drop strong {
  color: #185a40;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.12rem;
}

.decoder-loss-drop p {
  margin: 0 0 0 auto;
  color: $muted;
}

.decoder-controls {
  position: sticky;
  top: 14px;
}

.workspace--autoencoder {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.autoencoder-stage,
.autoencoder-controls {
  display: grid;
  gap: 14px;
}

.autoencoder-intro,
.autoencoder-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.autoencoder-intro,
.autoencoder-network-panel,
.autoencoder-reconstruction-panel,
.autoencoder-backward-panel,
.autoencoder-update-panel,
.autoencoder-controls {
  @include surface;
  padding: 16px;
}

.autoencoder-intro p:last-child,
.autoencoder-heading p,
.autoencoder-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.autoencoder-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(183, 121, 31, 0.3);
  border-radius: 999px;
  background: rgba(183, 121, 31, 0.09);
  color: #81520d;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.autoencoder-network-panel,
.autoencoder-reconstruction-panel,
.autoencoder-backward-panel,
.autoencoder-update-panel {
  display: grid;
  gap: 14px;
}

.autoencoder-heading > code {
  flex: 0 0 auto;
  padding: 6px 9px;
  border-radius: 6px;
  background: rgba(183, 121, 31, 0.1);
  color: #81520d;
}

.autoencoder-network {
  display: grid;
  grid-template-columns: minmax(100px, 0.8fr) auto minmax(160px, 1fr) auto minmax(110px, 0.8fr) auto minmax(160px, 1fr);
  gap: 8px;
  align-items: center;
}

.autoencoder-input-stack,
.autoencoder-encoder-stack,
.autoencoder-output-stack {
  display: grid;
  gap: 7px;
  min-width: 0;
}

.autoencoder-input-stack > small,
.autoencoder-encoder-stack > small,
.autoencoder-output-stack > small,
.autoencoder-bottleneck small,
.autoencoder-reconstruction-flow small,
.autoencoder-loss-badge small,
.autoencoder-branch-gradients small,
.autoencoder-gradient-grid small,
.autoencoder-parameter-grid small,
.autoencoder-loss-drop small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.autoencoder-input-stack > div,
.autoencoder-encoder-stack,
.autoencoder-output-stack > button {
  min-width: 0;
  padding: 9px;
  border: 1px solid $line;
  border-radius: 6px;
  background: $paper;
}

.autoencoder-input-stack > div {
  display: flex;
  justify-content: space-between;
  gap: 8px;
}

.autoencoder-encoder-stack code {
  overflow-wrap: anywhere;
  white-space: normal;
}

.autoencoder-bottleneck {
  display: grid;
  place-items: center;
  gap: 3px;
  min-height: 130px;
  padding: 13px;
  border: 3px solid rgba(183, 121, 31, 0.38);
  border-radius: 50%;
  background: rgba(183, 121, 31, 0.08);
  text-align: center;
}

.autoencoder-bottleneck strong {
  color: #81520d;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.45rem;
}

.autoencoder-bottleneck span {
  color: $muted;
  font-size: 0.72rem;
}

.autoencoder-output-stack > button {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 2px 8px;
  text-align: left;
}

.autoencoder-output-stack > button small {
  grid-column: 1 / -1;
  color: $muted;
}

.autoencoder-output-stack > button[aria-pressed="true"] {
  border-color: rgba(183, 121, 31, 0.55);
  background: rgba(183, 121, 31, 0.1);
  color: #674108;
}

.autoencoder-arrow {
  color: $muted;
  font-weight: 850;
}

.autoencoder-loss-badge {
  display: grid;
  gap: 2px;
  min-width: 150px;
  padding: 8px 10px;
  border-left: 3px solid rgba(194, 65, 59, 0.5);
  background: rgba(194, 65, 59, 0.05);
}

.autoencoder-loss-badge strong {
  color: #9f342f;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.18rem;
}

.autoencoder-reconstruction-flow {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr) auto) minmax(0, 1fr) auto minmax(0, 1.15fr);
  gap: 7px;
  align-items: stretch;
}

.autoencoder-reconstruction-flow > div,
.autoencoder-gradient-grid > div,
.autoencoder-parameter-grid > div {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: $paper;
}

.autoencoder-reconstruction-flow > span {
  align-self: center;
  color: $muted;
  font-weight: 850;
}

.autoencoder-reconstruction-flow code,
.autoencoder-gradient-grid code,
.autoencoder-parameter-grid code {
  overflow-wrap: anywhere;
  white-space: normal;
}

.autoencoder-reconstruction-result {
  border-color: rgba(35, 122, 87, 0.3) !important;
  background: rgba(35, 122, 87, 0.055) !important;
}

.autoencoder-error-result {
  border-color: rgba(194, 65, 59, 0.28) !important;
  background: rgba(194, 65, 59, 0.045) !important;
}

.autoencoder-branch-gradients {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr)) auto minmax(130px, 0.8fr);
  gap: 8px;
  align-items: stretch;
}

.autoencoder-branch-gradients > button,
.autoencoder-bottleneck-gradient {
  display: grid;
  align-content: center;
  gap: 4px;
  min-width: 0;
  padding: 10px;
  border: 1px solid rgba(109, 91, 208, 0.2);
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.035);
  text-align: left;
}

.autoencoder-branch-gradients > button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.55);
  background: rgba(109, 91, 208, 0.1);
  color: #4f3da9;
}

.autoencoder-branch-gradients > span {
  align-self: center;
  color: $muted;
  font-weight: 850;
}

.autoencoder-bottleneck-gradient {
  border-color: rgba(183, 121, 31, 0.36);
  background: rgba(183, 121, 31, 0.07);
}

.autoencoder-bottleneck-gradient strong {
  color: #81520d;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.14rem;
}

.autoencoder-gradient-grid,
.autoencoder-parameter-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 7px;
}

.autoencoder-gradient-audit {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 11px;
  border: 1px solid rgba(109, 91, 208, 0.24);
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.045);
}

.autoencoder-gradient-audit code {
  color: $muted;
}

.autoencoder-gradient-audit strong {
  margin-left: auto;
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.autoencoder-loss-drop {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px;
  border-left: 4px solid rgba(35, 122, 87, 0.52);
  background: rgba(35, 122, 87, 0.055);
}

.autoencoder-loss-drop > div {
  display: grid;
  gap: 2px;
}

.autoencoder-loss-drop strong {
  color: #185a40;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.12rem;
}

.autoencoder-loss-drop p {
  margin: 0 0 0 auto;
  color: $muted;
}

.autoencoder-controls {
  position: sticky;
  top: 14px;
}

.representation-workbench {
  display: grid;
  gap: 12px;
}

.representation-lab-switch {
  @include surface;
  display: flex;
  gap: 8px;
  padding: 8px;
}

.representation-lab-switch button,
.variational-gradient-targets button,
.variational-beta-buttons button {
  padding: 8px 11px;
  border: 1px solid $line;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.72);
  font-weight: 800;
}

.representation-lab-switch button[aria-pressed="true"],
.variational-gradient-targets button[aria-pressed="true"],
.variational-beta-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.55);
  background: rgba(109, 91, 208, 0.1);
  color: #4f3da9;
}

.workspace--variational {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.variational-stage,
.variational-controls {
  display: grid;
  gap: 14px;
}

.variational-intro,
.variational-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.variational-intro,
.variational-flow-panel,
.variational-objective-panel,
.variational-gradient-panel,
.variational-update-panel,
.variational-controls {
  @include surface;
  padding: 16px;
}

.variational-intro p:last-child,
.variational-heading p,
.variational-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.variational-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(109, 91, 208, 0.3);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.09);
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.variational-flow-panel,
.variational-objective-panel,
.variational-gradient-panel,
.variational-update-panel {
  display: grid;
  gap: 14px;
}

.variational-heading > code {
  flex: 0 0 auto;
  padding: 6px 9px;
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.09);
  color: #4f3da9;
}

.variational-flow {
  display: grid;
  grid-template-columns: minmax(95px, 0.6fr) auto minmax(210px, 1.35fr) auto minmax(170px, 1fr) auto minmax(180px, 1fr);
  gap: 8px;
  align-items: stretch;
}

.variational-scalar-node,
.variational-distribution-node,
.variational-sample-node,
.variational-objective-equation > div,
.variational-gradient-routes > div,
.variational-gradient-grid > div,
.variational-parameter-grid > div {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: $paper;
}

.variational-scalar-node small,
.variational-distribution-node small,
.variational-sample-node small,
.variational-objective-equation small,
.variational-loss-badge small,
.variational-gradient-routes small,
.variational-gradient-grid small,
.variational-parameter-grid small,
.variational-loss-drop small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.variational-scalar-node code,
.variational-distribution-node code,
.variational-sample-node code,
.variational-objective-equation code,
.variational-gradient-routes code,
.variational-gradient-grid code,
.variational-parameter-grid code {
  overflow-wrap: anywhere;
  white-space: normal;
}

.variational-sample-node {
  border-color: rgba(109, 91, 208, 0.34);
  background: rgba(109, 91, 208, 0.055);
}

.variational-sample-node strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.25rem;
}

.variational-sample-node span,
.variational-objective-equation span,
.variational-gradient-routes span {
  color: $muted;
  font-size: 0.73rem;
}

.variational-scalar-node--output {
  border-color: rgba(35, 122, 87, 0.3);
  background: rgba(35, 122, 87, 0.055);
}

.variational-scalar-node--output strong {
  color: #185a40;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.variational-arrow {
  align-self: center;
  color: $muted;
  font-weight: 850;
}

.variational-loss-badge {
  display: grid;
  gap: 2px;
  min-width: 135px;
  padding: 8px 10px;
  border-left: 3px solid rgba(194, 65, 59, 0.5);
  background: rgba(194, 65, 59, 0.05);
}

.variational-loss-badge strong {
  color: #9f342f;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.18rem;
}

.variational-objective-equation {
  display: grid;
  grid-template-columns: minmax(150px, 1fr) auto minmax(210px, 1.35fr) auto minmax(70px, 0.45fr) auto minmax(150px, 1fr);
  gap: 8px;
  align-items: stretch;
}

.variational-objective-equation > span,
.variational-gradient-routes > span {
  align-self: center;
  color: $muted;
  font-weight: 850;
}

.variational-objective-equation strong,
.variational-gradient-routes strong {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.16rem;
}

.variational-beta-node {
  place-items: center;
  border-color: rgba(183, 121, 31, 0.32) !important;
  background: rgba(183, 121, 31, 0.07) !important;
  text-align: center;
}

.variational-beta-node strong {
  color: #81520d;
}

.variational-total-node {
  border-color: rgba(194, 65, 59, 0.28) !important;
  background: rgba(194, 65, 59, 0.045) !important;
}

.variational-total-node strong {
  color: #9f342f;
}

.variational-gradient-targets,
.variational-beta-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}

.variational-gradient-routes {
  display: grid;
  grid-template-columns: minmax(150px, 1fr) auto minmax(170px, 1fr) auto minmax(180px, 1.1fr);
  gap: 8px;
  align-items: stretch;
}

.variational-combined-gradient {
  border-color: rgba(109, 91, 208, 0.38) !important;
  background: rgba(109, 91, 208, 0.07) !important;
}

.variational-combined-gradient strong {
  color: #4f3da9;
}

.variational-gradient-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 7px;
}

.variational-parameter-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 7px;
}

.variational-audit-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 11px;
  border: 1px solid rgba(109, 91, 208, 0.24);
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.045);
}

.variational-audit-row code {
  color: $muted;
}

.variational-audit-row strong {
  margin-left: auto;
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.variational-loss-drop {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px;
  border-left: 4px solid rgba(35, 122, 87, 0.52);
  background: rgba(35, 122, 87, 0.055);
}

.variational-loss-drop > div {
  display: grid;
  gap: 2px;
}

.variational-loss-drop strong {
  color: #185a40;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.12rem;
}

.variational-loss-drop p {
  margin: 0 0 0 auto;
  color: $muted;
}

.variational-controls {
  position: sticky;
  top: 14px;
}

.variational-selected-summary {
  display: grid;
  gap: 4px;
  padding: 11px;
  border-left: 3px solid rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.045);
}

.variational-selected-summary small,
.variational-selected-summary span {
  color: $muted;
}

.variational-selected-summary strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.7rem;
}

.workspace--gan {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.gan-stage,
.gan-controls {
  display: grid;
  gap: 14px;
}

.gan-intro,
.gan-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.gan-intro,
.gan-sample-panel,
.gan-objective-panel,
.gan-gradient-panel,
.gan-update-panel,
.gan-controls {
  @include surface;
  padding: 16px;
}

.gan-intro p:last-child,
.gan-heading p,
.gan-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.gan-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(35, 122, 87, 0.3);
  border-radius: 999px;
  background: rgba(35, 122, 87, 0.08);
  color: #185a40;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.gan-sample-panel,
.gan-objective-panel,
.gan-gradient-panel,
.gan-update-panel {
  display: grid;
  gap: 14px;
}

.gan-heading > code {
  flex: 0 1 390px;
  padding: 6px 9px;
  border-radius: 6px;
  background: rgba(35, 122, 87, 0.07);
  color: #185a40;
  overflow-wrap: anywhere;
  white-space: normal;
}

.gan-number-line {
  position: relative;
  min-height: 135px;
  margin: 10px 22px 0;
  border-bottom: 3px solid #334155;
}

.gan-number-line__axis {
  position: absolute;
  inset: auto 0 -25px;
  display: flex;
  justify-content: space-between;
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.72rem;
}

.gan-number-line__marker {
  position: absolute;
  bottom: 10px;
  display: grid;
  gap: 2px;
  min-width: 95px;
  padding: 8px;
  border-radius: 6px;
  transform: translateX(-50%);
  text-align: center;
}

.gan-number-line__marker::after {
  position: absolute;
  bottom: -13px;
  left: calc(50% - 5px);
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: currentColor;
  content: "";
}

.gan-number-line__marker small {
  color: $muted;
}

.gan-number-line__marker--fake {
  bottom: 10px;
  background: rgba(109, 91, 208, 0.09);
  color: #4f3da9;
}

.gan-number-line__marker--real {
  bottom: 70px;
  background: rgba(35, 122, 87, 0.09);
  color: #185a40;
}

.gan-number-line__marker--real::after {
  bottom: -73px;
  height: 70px;
  border-radius: 0;
  background: linear-gradient(currentColor, currentColor 2px, transparent 2px);
}

.gan-probability-grid,
.gan-update-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.gan-probability-grid > div,
.gan-player,
.gan-gradient-route > div,
.gan-update-grid > div,
.gan-freeze-key {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: $paper;
}

.gan-probability-grid small,
.gan-player small,
.gan-gradient-route small,
.gan-update-grid small,
.gan-freeze-key small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.gan-probability-grid code,
.gan-player code,
.gan-gradient-route code,
.gan-update-grid code,
.gan-freeze-key code {
  overflow-wrap: anywhere;
  white-space: normal;
}

.gan-probability-grid strong,
.gan-player strong,
.gan-gradient-route strong,
.gan-update-grid strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.gan-probability-grid strong {
  color: #185a40;
  font-size: 1.18rem;
}

.gan-objectives {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  gap: 10px;
  align-items: stretch;
}

.gan-player {
  border-left: 4px solid rgba(35, 122, 87, 0.38);
}

.gan-player--generator {
  border-left-color: rgba(109, 91, 208, 0.42);
}

.gan-player--active {
  border-color: rgba(35, 122, 87, 0.56);
  background: rgba(35, 122, 87, 0.07);
}

.gan-player--active.gan-player--generator {
  border-color: rgba(109, 91, 208, 0.56);
  background: rgba(109, 91, 208, 0.08);
}

.gan-player span,
.gan-gradient-route span,
.gan-update-grid span,
.gan-selected-summary span {
  color: $muted;
  font-size: 0.73rem;
}

.gan-versus,
.gan-gradient-route > span {
  align-self: center;
  color: $muted;
  font-weight: 850;
  text-transform: uppercase;
}

.gan-gradient-placeholder {
  padding: 18px;
  border: 1px dashed rgba(109, 91, 208, 0.35);
  border-radius: 6px;
  background: rgba(109, 91, 208, 0.035);
  color: $muted;
  text-align: center;
}

.gan-gradient-route {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto minmax(0, 1.2fr);
  gap: 8px;
  align-items: stretch;
}

.gan-gradient-route__result {
  border-color: rgba(35, 122, 87, 0.38) !important;
  background: rgba(35, 122, 87, 0.06) !important;
}

.gan-gradient-route__result--generator {
  border-color: rgba(109, 91, 208, 0.42) !important;
  background: rgba(109, 91, 208, 0.07) !important;
}

.gan-update-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.gan-update-grid > div:first-child {
  border-left: 4px solid rgba(35, 122, 87, 0.42);
}

.gan-update-grid > div:last-child {
  border-left: 4px solid rgba(109, 91, 208, 0.45);
}

.gan-counterpush {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px;
  border-left: 4px solid rgba(183, 121, 31, 0.5);
  background: rgba(183, 121, 31, 0.07);
}

.gan-counterpush strong {
  color: #81520d;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.gan-counterpush p {
  margin: 0 0 0 auto;
  color: $muted;
}

.gan-controls {
  position: sticky;
  top: 14px;
}

.gan-phase-buttons {
  display: grid;
  gap: 8px;
}

.gan-phase-buttons button {
  display: grid;
  gap: 2px;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.72);
  text-align: left;
}

.gan-phase-buttons button span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.72rem;
}

.gan-phase-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.55);
  background: rgba(109, 91, 208, 0.1);
  color: #4f3da9;
}

.gan-selected-summary {
  display: grid;
  gap: 4px;
  padding: 11px;
  border-left: 3px solid rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.045);
}

.gan-selected-summary small {
  color: $muted;
}

.gan-selected-summary strong {
  color: #4f3da9;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.15rem;
}

.workspace--diffusion {
  display: grid;
  grid-template-columns: minmax(700px, 1fr) 300px;
  gap: 14px;
  align-items: start;
}

.diffusion-stage,
.diffusion-controls {
  display: grid;
  gap: 14px;
}

.diffusion-intro,
.diffusion-heading {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 18px;
}

.diffusion-intro,
.diffusion-forward-panel,
.diffusion-predict-panel,
.diffusion-gradient-panel,
.diffusion-reverse-panel,
.diffusion-controls {
  @include surface;
  padding: 16px;
}

.diffusion-intro p:last-child,
.diffusion-heading p,
.diffusion-controls > p {
  margin: 7px 0 0;
  color: $muted;
}

.diffusion-chip {
  flex: 0 0 auto;
  padding: 7px 10px;
  border: 1px solid rgba(43, 108, 176, 0.3);
  border-radius: 999px;
  background: rgba(43, 108, 176, 0.08);
  color: #235d98;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 750;
}

.diffusion-forward-panel,
.diffusion-predict-panel,
.diffusion-gradient-panel,
.diffusion-reverse-panel {
  display: grid;
  gap: 14px;
}

.diffusion-heading > code {
  flex: 0 1 350px;
  padding: 6px 9px;
  border-radius: 6px;
  background: rgba(43, 108, 176, 0.07);
  color: #235d98;
  overflow-wrap: anywhere;
  white-space: normal;
}

.diffusion-forward-lane {
  display: grid;
  grid-template-columns: minmax(130px, 0.7fr) repeat(2, minmax(230px, 1fr));
  gap: 9px;
  align-items: stretch;
}

.diffusion-forward-hop {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 8px;
  align-items: stretch;
}

.diffusion-forward-hop > span,
.diffusion-reverse-lane > span {
  align-self: center;
  color: $muted;
  font-size: 0.72rem;
  font-weight: 850;
  text-transform: uppercase;
}

.diffusion-state,
.diffusion-prediction-grid > div,
.diffusion-gradient-rows > div,
.diffusion-gradient-sum > div,
.diffusion-update-row > div,
.diffusion-reverse-step,
.diffusion-final-state,
.diffusion-coefficient-grid > div {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  padding: 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: $paper;
}

.diffusion-state small,
.diffusion-prediction-grid small,
.diffusion-gradient-rows small,
.diffusion-gradient-sum small,
.diffusion-update-row small,
.diffusion-reverse-step small,
.diffusion-final-state small,
.diffusion-coefficient-grid small,
.diffusion-loss-badge small {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 750;
}

.diffusion-state code,
.diffusion-prediction-grid code,
.diffusion-gradient-rows code,
.diffusion-update-row code,
.diffusion-reverse-step code,
.diffusion-coefficient-grid code,
.diffusion-equation code {
  overflow-wrap: anywhere;
  white-space: normal;
}

.diffusion-state strong,
.diffusion-prediction-grid strong,
.diffusion-gradient-sum strong,
.diffusion-update-row strong,
.diffusion-reverse-step strong,
.diffusion-final-state strong,
.diffusion-coefficient-grid strong {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.diffusion-state span,
.diffusion-prediction-grid span,
.diffusion-gradient-rows span,
.diffusion-update-row span,
.diffusion-reverse-step span,
.diffusion-final-state span {
  color: $muted;
  font-size: 0.73rem;
}

.diffusion-state--noisy {
  border-color: rgba(43, 108, 176, 0.26);
  background: rgba(43, 108, 176, 0.045);
}

.diffusion-state--active,
.diffusion-reverse-step--active {
  border-color: rgba(43, 108, 176, 0.62);
  box-shadow: inset 0 0 0 2px rgba(43, 108, 176, 0.09);
  background: rgba(43, 108, 176, 0.09);
}

.diffusion-state--active strong,
.diffusion-reverse-step--active strong {
  color: #235d98;
}

.diffusion-coefficient-grid,
.diffusion-prediction-grid,
.diffusion-gradient-rows {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.diffusion-forward-note {
  margin: 0;
  padding: 9px 11px;
  border-left: 3px solid rgba(183, 121, 31, 0.45);
  background: rgba(183, 121, 31, 0.055);
  color: $muted;
}

.diffusion-loss-badge {
  display: grid;
  gap: 2px;
  min-width: 150px;
  padding: 8px 10px;
  border-left: 3px solid rgba(194, 65, 59, 0.5);
  background: rgba(194, 65, 59, 0.05);
}

.diffusion-loss-badge strong {
  color: #9f342f;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.18rem;
}

.diffusion-equation {
  padding: 10px;
  border: 1px dashed rgba(43, 108, 176, 0.35);
  border-radius: 6px;
  background: rgba(43, 108, 176, 0.035);
  text-align: center;
}

.diffusion-prediction-grid > div {
  border-left: 4px solid rgba(43, 108, 176, 0.38);
}

.diffusion-gradient-sum {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.diffusion-gradient-sum strong {
  color: #4f3da9;
  font-size: 1.15rem;
}

.diffusion-update-row {
  display: grid;
  grid-template-columns: 1.3fr 0.8fr 1fr;
  gap: 8px;
}

.diffusion-loss-drop {
  border-color: rgba(35, 122, 87, 0.35) !important;
  background: rgba(35, 122, 87, 0.06) !important;
}

.diffusion-loss-drop strong {
  color: #185a40;
  font-size: 1.12rem;
}

.diffusion-reverse-lane {
  display: grid;
  grid-template-columns: minmax(110px, 0.7fr) auto minmax(150px, 1fr) auto minmax(150px, 1fr) auto minmax(140px, 0.9fr);
  gap: 8px;
  align-items: stretch;
}

.diffusion-reverse-step {
  border-color: rgba(109, 91, 208, 0.3);
  background: rgba(109, 91, 208, 0.045);
}

.diffusion-final-state {
  border-color: rgba(35, 122, 87, 0.38);
  background: rgba(35, 122, 87, 0.06);
}

.diffusion-final-state strong {
  color: #185a40;
  font-size: 1.2rem;
}

.diffusion-controls {
  position: sticky;
  top: 14px;
}

.diffusion-phase-buttons {
  display: grid;
  gap: 7px;
}

.diffusion-phase-buttons button {
  display: grid;
  gap: 2px;
  padding: 9px 10px;
  border: 1px solid $line;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.72);
  text-align: left;
}

.diffusion-phase-buttons button span {
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.72rem;
}

.diffusion-phase-buttons button[aria-pressed="true"] {
  border-color: rgba(43, 108, 176, 0.58);
  background: rgba(43, 108, 176, 0.1);
  color: #235d98;
}

.diffusion-selected-summary {
  display: grid;
  gap: 4px;
  padding: 11px;
  border-left: 3px solid rgba(43, 108, 176, 0.48);
  background: rgba(43, 108, 176, 0.05);
}

.diffusion-selected-summary small,
.diffusion-selected-summary span {
  color: $muted;
}

.diffusion-selected-summary strong {
  color: #235d98;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 1.15rem;
}

@media (max-width: 1360px) {
  .app-header,
  .header-actions {
    display: grid;
    align-items: start;
  }

  .formula {
    min-width: 0;
    text-align: left;
  }
}

@media (max-width: 1180px) {
  .workspace--lab,
  .workspace--hidden {
    grid-template-columns: 270px minmax(420px, 1fr);
  }

  .metrics,
  .microscope-controls,
  .optimization-controls,
  .convolution-controls {
    grid-column: 1 / -1;
  }

  .image-cnn-controls {
    grid-column: 1 / -1;
  }

  .residual-controls {
    grid-column: 1 / -1;
  }

  .recurrent-controls {
    grid-column: 1 / -1;
  }

  .workspace--microscope,
  .workspace--optimization,
  .workspace--convolution,
  .workspace--image-cnn,
  .workspace--residual,
  .workspace--recurrent,
  .workspace--bptt,
    .workspace--gates,
    .workspace--attention,
    .workspace--attention-softmax,
    .workspace--multi-head,
    .workspace--decoder,
    .workspace--autoencoder,
    .workspace--variational,
    .workspace--gan,
    .workspace--diffusion {
    grid-template-columns: 1fr;
  }

  .attention-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .attention-softmax-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .multi-head-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .decoder-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .autoencoder-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .variational-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .gan-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .diffusion-controls {
    position: static;
    grid-column: 1 / -1;
  }

  .gate-controls {
    grid-column: 1 / -1;
  }
}

@media (max-width: 820px) {
  .app {
    width: min(100vw - 16px, 720px);
    padding-top: 12px;
  }

  .app-header,
  .lab-intro,
  .header-actions {
    display: grid;
  }

  .formula {
    min-width: 0;
    text-align: left;
  }

  .workspace--lab,
  .workspace--hidden,
  .workspace--microscope,
  .workspace--optimization,
  .workspace--convolution,
  .workspace--image-cnn,
  .workspace--residual,
  .workspace--recurrent,
  .workspace--bptt,
    .workspace--gates,
    .workspace--attention,
    .workspace--attention-softmax,
    .workspace--multi-head,
    .workspace--decoder,
    .workspace--autoencoder,
    .workspace--variational,
    .workspace--gan,
    .workspace--diffusion {
    grid-template-columns: 1fr;
  }

  .convolution-intro,
  .mac-heading,
  .training-heading,
  .image-cnn-intro,
  .image-stage-heading,
  .residual-intro,
  .residual-panel-heading,
  .recurrent-intro,
  .recurrent-panel-heading,
  .recurrent-arithmetic-heading,
  .bptt-intro,
  .bptt-panel-heading,
  .bptt-arithmetic-heading,
  .gate-intro,
  .gate-panel-heading,
  .attention-intro,
    .attention-panel-heading,
    .attention-softmax-intro,
    .attention-softmax-heading,
    .multi-head-intro,
    .multi-head-heading,
    .decoder-intro,
    .decoder-heading,
    .autoencoder-intro,
    .autoencoder-heading,
    .variational-intro,
    .variational-heading,
    .gan-intro,
    .gan-heading,
    .diffusion-intro,
    .diffusion-heading {
    display: grid;
    align-items: start;
  }

  .gradient-check-badge {
    justify-self: start;
  }

  .loss-flow {
    align-items: stretch;
  }

  .loss-flow > div {
    min-width: 0;
  }

  .convolution-mode-chip {
    justify-self: start;
  }

  .lab-rail {
    max-height: 320px;
  }

  .field-grid {
    grid-template-columns: 1fr;
  }

  .table-row {
    grid-template-columns: 1fr;
  }

  .mode-toggle {
    grid-template-columns: repeat(2, 1fr);
  }

  .image-pipeline {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .image-stage-button:last-child {
    grid-column: 1 / -1;
  }

  .image-channel-grid,
  .channel-math-grid,
  .image-map-pair,
  .pooling-grid,
  .normalization-flow,
  .activation-flow {
    grid-template-columns: 1fr;
  }

  .activation-flow > span,
  .pooling-card > span {
    transform: rotate(90deg);
    justify-self: center;
  }

  .pooling-card {
    grid-template-columns: 1fr;
  }

  .channel-reduction {
    grid-template-columns: repeat(3, auto);
  }

  .channel-reduction > span:nth-of-type(3),
  .channel-reduction__result {
    grid-column: auto;
  }

  .residual-skip-lane {
    display: grid;
    justify-items: center;
    text-align: center;
  }

  .residual-addition {
    grid-template-columns: repeat(3, auto);
  }

  .receptive-summary {
    grid-template-columns: 1fr;
  }

  .receptive-summary code {
    overflow-x: auto;
    white-space: nowrap;
  }

  .shared-parameter-strip {
    flex-wrap: wrap;
  }

  .recurrent-chain {
    grid-template-columns: 1fr;
    justify-items: stretch;
  }

  .recurrent-initial-node,
  .recurrent-cell {
    min-height: 0;
  }

  .recurrent-connector {
    min-height: 38px;
  }

  .recurrent-connector span {
    transform: rotate(90deg);
  }

  .recurrent-equation {
    grid-template-columns: 1fr;
  }

  .recurrent-equation > span {
    justify-self: center;
  }

  .recurrent-equation > span:last-of-type {
    transform: rotate(90deg);
  }

  .bptt-forward-lane,
  .bptt-backward-lane,
  .bptt-audit-grid,
  .bptt-local-gradients {
    grid-template-columns: 1fr;
  }

  .bptt-equation {
    grid-template-columns: 1fr;
  }

  .bptt-equation > span {
    justify-self: center;
  }

  .gate-model-lane,
  .gate-flow,
  .gate-flow--lstm {
    grid-template-columns: 1fr;
  }

  .attention-projection-head {
    display: none;
  }

  .attention-projection-row,
  .attention-score-layout,
  .attention-vector-pair {
    grid-template-columns: 1fr;
  }

  .attention-projection-row > div:first-child {
    grid-template-columns: auto 1fr;
  }

  .attention-score-grid {
    grid-template-columns: minmax(54px, 0.7fr) repeat(3, minmax(60px, 1fr));
  }

  .attention-score-cell {
    min-height: 58px;
    padding: 4px;
    font-size: 0.98rem;
  }

  .attention-dot-equation {
    align-items: start;
    flex-direction: column;
  }

  .attention-weight-grid {
    grid-template-columns: minmax(58px, 0.7fr) repeat(3, minmax(62px, 1fr));
  }

  .attention-weight-row-button,
  .attention-weight-cell {
    min-height: 58px;
    padding: 4px;
    font-size: 0.76rem;
  }

  .attention-normalize-flow,
  .attention-value-lanes {
    grid-template-columns: 1fr;
  }

  .attention-normalize-flow > span {
    justify-self: center;
    transform: rotate(90deg);
  }

  .multi-head-lanes,
  .multi-head-join-flow,
  .multi-head-norm-flow {
    grid-template-columns: 1fr;
  }

  .multi-head-join-flow > span {
    justify-self: center;
    transform: rotate(90deg);
  }

  .multi-head-value-row {
    grid-template-columns: 1fr;
  }

  .decoder-position-lanes,
  .decoder-vocabulary-grid,
  .decoder-softmax-trace,
  .decoder-forward-flow,
  .decoder-gradient-flow,
  .decoder-update-grid {
    grid-template-columns: 1fr;
  }

  .decoder-position-button {
    grid-template-columns: 1fr;
  }

  .decoder-position-button > i {
    justify-self: center;
    transform: rotate(90deg);
  }

  .decoder-chip {
    justify-self: start;
  }

  .decoder-forward-flow > span,
  .decoder-gradient-flow > span {
    justify-self: center;
    transform: rotate(90deg);
  }

  .decoder-loss-drop {
    align-items: start;
    flex-direction: column;
  }

  .decoder-gradient-audit {
    align-items: start;
    flex-direction: column;
  }

  .decoder-gradient-audit strong {
    margin-left: 0;
  }

  .decoder-loss-drop p {
    margin-left: 0;
  }

  .autoencoder-network,
  .autoencoder-reconstruction-flow,
  .autoencoder-branch-gradients,
  .autoencoder-gradient-grid,
  .autoencoder-parameter-grid {
    grid-template-columns: 1fr;
  }

  .autoencoder-arrow,
  .autoencoder-reconstruction-flow > span {
    justify-self: center;
    transform: rotate(90deg);
  }

  .autoencoder-branch-gradients > span {
    justify-self: center;
  }

  .autoencoder-bottleneck {
    justify-self: center;
    min-width: 140px;
  }

  .autoencoder-chip {
    justify-self: start;
  }

  .autoencoder-gradient-audit,
  .autoencoder-loss-drop {
    align-items: start;
    flex-direction: column;
  }

  .autoencoder-gradient-audit strong,
  .autoencoder-loss-drop p {
    margin-left: 0;
  }

  .representation-lab-switch {
    display: grid;
    grid-template-columns: 1fr;
  }

  .variational-flow,
  .variational-objective-equation,
  .variational-gradient-routes,
  .variational-gradient-grid,
  .variational-parameter-grid {
    grid-template-columns: 1fr;
  }

  .variational-arrow {
    justify-self: center;
    transform: rotate(90deg);
  }

  .variational-objective-equation > span,
  .variational-gradient-routes > span {
    justify-self: center;
  }

  .variational-chip {
    justify-self: start;
  }

  .variational-audit-row,
  .variational-loss-drop {
    align-items: start;
    flex-direction: column;
  }

  .variational-audit-row strong,
  .variational-loss-drop p {
    margin-left: 0;
  }

  .gan-chip {
    justify-self: start;
  }

  .gan-probability-grid,
  .gan-objectives,
  .gan-gradient-route,
  .gan-update-grid {
    grid-template-columns: 1fr;
  }

  .gan-versus,
  .gan-gradient-route > span {
    justify-self: center;
  }

  .gan-counterpush {
    align-items: start;
    flex-direction: column;
  }

  .gan-counterpush p {
    margin-left: 0;
  }

  .diffusion-chip {
    justify-self: start;
  }

  .diffusion-forward-lane,
  .diffusion-forward-hop,
  .diffusion-coefficient-grid,
  .diffusion-prediction-grid,
  .diffusion-gradient-rows,
  .diffusion-gradient-sum,
  .diffusion-update-row,
  .diffusion-reverse-lane {
    grid-template-columns: 1fr;
  }

  .diffusion-forward-hop > span,
  .diffusion-reverse-lane > span {
    justify-self: center;
  }

  .gate-model-label {
    padding: 0 0 9px;
    border-right: 0;
    border-bottom: 2px solid rgba(109, 91, 208, 0.25);
  }

  .gate-state-node,
  .gate-candidate-node,
  .gate-cell-node,
  .gate-result-node,
  .gate-node {
    min-height: 0;
  }

  .phase-strip {
    grid-template-columns: repeat(2, 1fr);
  }

  .microscope-focus {
    grid-template-columns: 1fr;
  }

  .microscope-focus > p {
    grid-column: auto;
  }

  .signal-pipeline {
    grid-template-columns: repeat(2, minmax(130px, 1fr));
    overflow: visible;
  }

  .derivative-panel {
    grid-template-columns: 1fr;
  }

  .derivative-times,
  .derivative-equals {
    display: none;
  }

  .before-after {
    grid-template-columns: 1fr;
  }

  .landscape-equation,
  .strategy-grid {
    grid-template-columns: 1fr;
  }

  .gradient-check-grid {
    grid-template-columns: 72px repeat(3, minmax(0, 1fr));
    min-width: 0;
  }

  .gradient-check-grid > * {
    padding: 6px 3px;
    font-size: 0.68rem;
  }

  .update-arrow {
    transform: rotate(90deg);
  }
}

.deep-training-workbench {
  display: grid;
  gap: 14px;
}

.deep-training-switch {
  display: flex;
  gap: 8px;
  padding: 0 24px;
}

.deep-training-switch button {
  border: 1px solid $line;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.72);
  color: $ink;
  padding: 10px 16px;
  font: inherit;
  font-weight: 750;
}

.deep-training-switch button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.42);
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
}

.workspace--gradient-flow {
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.gradient-flow-stage {
  display: grid;
  gap: 16px;
  min-width: 0;
}

.gradient-flow-intro,
.gradient-forward-panel,
.gradient-backward-panel,
.gradient-arithmetic-panel,
.gradient-chain-panel,
.gradient-comparison-panel,
.gradient-flow-controls {
  @include surface;
}

.gradient-flow-intro,
.gradient-forward-panel,
.gradient-backward-panel,
.gradient-arithmetic-panel,
.gradient-chain-panel,
.gradient-comparison-panel {
  padding: 18px;
}

.gradient-flow-intro {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.gradient-flow-intro h2,
.gradient-flow-controls h2 {
  margin: 0;
}

.gradient-flow-intro p:not(.eyebrow),
.gradient-flow-controls > p,
.gradient-chain-panel > p {
  color: $muted;
  line-height: 1.55;
}

.gradient-flow-chip {
  flex: 0 0 auto;
  border: 1px solid currentColor;
  border-radius: 999px;
  padding: 8px 12px;
  font-weight: 850;
  text-transform: uppercase;
}

.gradient-flow-chip--vanishing {
  background: rgba(37, 99, 235, 0.08);
  color: #234c9f;
}

.gradient-flow-chip--stable {
  background: rgba(35, 122, 87, 0.08);
  color: #1d6849;
}

.gradient-flow-chip--exploding {
  background: rgba(194, 65, 59, 0.08);
  color: #9b3131;
}

.gradient-forward-panel,
.gradient-backward-panel,
.gradient-arithmetic-panel,
.gradient-chain-panel,
.gradient-comparison-panel {
  display: grid;
  gap: 14px;
}

.gradient-forward-lane {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 8px;
}

.gradient-forward-lane > div {
  display: grid;
  align-content: center;
  gap: 5px;
  min-width: 0;
  min-height: 92px;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.055);
  padding: 10px;
  text-align: center;
}

.gradient-forward-lane span,
.gradient-backward-lane span,
.gradient-backward-lane small,
.gradient-input-node span {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.gradient-forward-lane code,
.gradient-forward-lane strong,
.gradient-backward-lane strong,
.gradient-backward-lane code,
.gradient-input-node strong,
.gradient-chain-panel strong,
.gradient-chain-panel code,
.gradient-audit code,
.gradient-comparison-grid code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.gradient-forward-lane code {
  color: #234c9f;
  font-size: 0.7rem;
}

.gradient-forward-lane .gradient-loss-node {
  border-color: rgba(194, 65, 59, 0.22);
  background: rgba(194, 65, 59, 0.07);
}

.gradient-backward-lane {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
}

.gradient-backward-lane button,
.gradient-input-node {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid rgba(109, 91, 208, 0.22);
  border-radius: 9px;
  background: rgba(109, 91, 208, 0.055);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.gradient-backward-lane button:hover,
.gradient-backward-lane button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.55);
  background: rgba(109, 91, 208, 0.13);
}

.gradient-backward-lane code {
  color: #4b3e9d;
  font-size: 0.72rem;
}

.gradient-input-node {
  align-content: center;
  border-color: rgba(35, 122, 87, 0.28);
  background: rgba(35, 122, 87, 0.075);
}

.gradient-equation-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(190px, 0.65fr);
  gap: 8px;
  align-items: center;
}

.gradient-equation-grid code,
.gradient-equation-grid span {
  min-width: 0;
  border-radius: 8px;
  padding: 10px;
}

.gradient-equation-grid code {
  background: rgba(37, 99, 235, 0.07);
  color: #234c9f;
  font-family: "SFMono-Regular", Consolas, monospace;
}

.gradient-equation-grid span {
  color: $muted;
  font-size: 0.75rem;
}

.gradient-chain-equation {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr)) auto minmax(90px, 1fr);
  align-items: center;
  gap: 7px;
}

.gradient-chain-equation code,
.gradient-chain-equation strong {
  border-radius: 8px;
  padding: 10px;
  text-align: center;
}

.gradient-chain-equation code {
  background: rgba(109, 91, 208, 0.08);
  color: #4b3e9d;
}

.gradient-chain-equation span {
  color: $muted;
  font-weight: 850;
}

.gradient-chain-equation strong {
  background: rgba(35, 122, 87, 0.08);
  color: #1d6849;
}

.gradient-audit {
  display: grid;
  grid-template-columns: auto minmax(90px, 1fr) auto minmax(90px, 1fr);
  align-items: center;
  gap: 8px;
  border-top: 1px solid $line;
  padding-top: 12px;
}

.gradient-audit span {
  color: $muted;
  font-size: 0.72rem;
  font-weight: 800;
  text-transform: uppercase;
}

.gradient-audit code {
  border-radius: 7px;
  background: rgba(35, 122, 87, 0.075);
  padding: 8px;
}

.gradient-comparison-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 9px;
}

.gradient-comparison-grid article {
  display: grid;
  gap: 7px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  padding: 11px;
}

.gradient-comparison-grid article.is-selected {
  border-color: rgba(109, 91, 208, 0.45);
  background: rgba(109, 91, 208, 0.055);
}

.gradient-comparison-grid article > span,
.gradient-comparison-grid small {
  color: $muted;
  font-size: 0.7rem;
  text-transform: uppercase;
}

.gradient-comparison-grid i {
  display: block;
  min-width: 2px;
  height: 8px;
  border-radius: 999px;
  background: linear-gradient(90deg, $blue, $red);
}

.gradient-flow-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 10px;
  padding: 16px;
}

.gradient-scenario-buttons {
  display: grid;
  gap: 7px;
}

.gradient-scenario-buttons button {
  display: grid;
  gap: 4px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.gradient-scenario-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.48);
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
}

.gradient-scenario-buttons span,
.gradient-scenario-buttons code {
  color: $muted;
  font-size: 0.7rem;
}

.gradient-flow-reading {
  display: grid;
  gap: 7px;
  margin-top: 6px;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.065);
  padding: 11px;
}

.gradient-flow-reading h2,
.gradient-flow-reading p {
  margin: 0;
}

@media (max-width: 1180px) {
  .workspace--gradient-flow {
    grid-template-columns: 1fr;
  }

  .gradient-flow-controls {
    position: static;
  }

  .gradient-scenario-buttons {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .deep-training-switch,
  .gradient-flow-intro,
  .gradient-forward-panel .panel-heading,
  .gradient-backward-panel .panel-heading,
  .gradient-arithmetic-panel .panel-heading,
  .gradient-chain-panel .panel-heading,
  .gradient-comparison-panel .panel-heading {
    display: grid;
  }

  .gradient-flow-chip {
    justify-self: start;
  }

  .deep-training-switch,
  .gradient-forward-lane,
  .gradient-backward-lane,
  .gradient-equation-grid,
  .gradient-chain-equation,
  .gradient-audit,
  .gradient-comparison-grid,
  .gradient-scenario-buttons {
    grid-template-columns: 1fr;
  }

  .gradient-chain-equation span {
    justify-self: center;
  }
}

.workspace--stabilizers {
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.stabilizer-stage {
  display: grid;
  gap: 16px;
  min-width: 0;
}

.stabilizer-intro,
.stabilizer-common-panel,
.stabilizer-comparison-panel,
.stabilizer-forward-panel,
.stabilizer-backward-panel,
.stabilizer-arithmetic-panel,
.stabilizer-audit-panel,
.stabilizer-controls {
  @include surface;
}

.stabilizer-intro,
.stabilizer-common-panel,
.stabilizer-comparison-panel,
.stabilizer-forward-panel,
.stabilizer-backward-panel,
.stabilizer-arithmetic-panel,
.stabilizer-audit-panel {
  padding: 18px;
}

.stabilizer-intro {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.stabilizer-intro h2,
.stabilizer-controls h2 {
  margin: 0;
}

.stabilizer-intro p:not(.eyebrow),
.stabilizer-controls > p {
  color: $muted;
  line-height: 1.55;
}

.stabilizer-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(35, 122, 87, 0.4);
  border-radius: 999px;
  background: rgba(35, 122, 87, 0.08);
  color: #1d6849;
  padding: 8px 12px;
  font-weight: 850;
  text-transform: uppercase;
}

.stabilizer-common-panel,
.stabilizer-comparison-panel,
.stabilizer-forward-panel,
.stabilizer-backward-panel,
.stabilizer-arithmetic-panel,
.stabilizer-audit-panel {
  display: grid;
  gap: 14px;
}

.stabilizer-common-flow {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) minmax(0, 1fr);
  align-items: end;
  gap: 9px;
}

.stabilizer-flow-arrow,
.stabilizer-plus {
  align-self: center;
  color: $muted;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-weight: 850;
  text-align: center;
}

.stabilizer-vector {
  display: grid;
  gap: 7px;
  min-width: 0;
}

.stabilizer-vector > span {
  color: $muted;
  font-size: 0.7rem;
  font-weight: 800;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.stabilizer-vector > div {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 5px;
}

.stabilizer-vector code {
  display: grid;
  gap: 3px;
  min-width: 0;
  border: 1px solid rgba(37, 99, 235, 0.18);
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.055);
  color: #234c9f;
  padding: 9px 5px;
  font-family: "SFMono-Regular", Consolas, monospace;
  text-align: center;
}

.stabilizer-vector code small {
  color: $muted;
  font-size: 0.6rem;
}

.stabilizer-vector code.is-selected {
  border-color: rgba(194, 65, 59, 0.58);
  box-shadow: inset 0 -3px rgba(194, 65, 59, 0.13);
}

.stabilizer-vector--purple code {
  border-color: rgba(109, 91, 208, 0.2);
  background: rgba(109, 91, 208, 0.06);
  color: #4b3e9d;
}

.stabilizer-vector--green code {
  border-color: rgba(35, 122, 87, 0.22);
  background: rgba(35, 122, 87, 0.065);
  color: #1d6849;
}

.stabilizer-vector--red code {
  border-color: rgba(194, 65, 59, 0.2);
  background: rgba(194, 65, 59, 0.055);
  color: #9b3131;
}

.stabilizer-comparison-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 9px;
}

.stabilizer-comparison-grid button {
  display: grid;
  gap: 7px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.78);
  color: $ink;
  padding: 11px;
  text-align: left;
}

.stabilizer-comparison-grid button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.08);
}

.stabilizer-comparison-grid span,
.stabilizer-comparison-grid small {
  color: $muted;
  font-size: 0.7rem;
}

.stabilizer-comparison-grid code,
.stabilizer-comparison-grid small {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.stabilizer-comparison-grid code {
  overflow-wrap: anywhere;
  color: #4b3e9d;
  font-size: 0.67rem;
}

.stabilizer-mechanism-trace,
.stabilizer-gradient-flow {
  display: grid;
  gap: 12px;
}

.stabilizer-stat-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.stabilizer-stat-grid > div,
.stabilizer-dropout-compare > div {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 8px;
  background: rgba(37, 99, 235, 0.045);
  padding: 10px;
}

.stabilizer-stat-grid small,
.stabilizer-dropout-compare small,
.stabilizer-audit-grid small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  text-transform: uppercase;
}

.stabilizer-stat-grid strong,
.stabilizer-dropout-compare code,
.stabilizer-audit-grid code,
.stabilizer-formula,
.stabilizer-equations code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.stabilizer-dropout-compare {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.stabilizer-dropout-compare code {
  color: #1d6849;
  overflow-wrap: anywhere;
}

.stabilizer-formula {
  border-radius: 8px;
  background: rgba(109, 91, 208, 0.07);
  color: #4b3e9d;
  padding: 11px;
  overflow-wrap: anywhere;
}

.stabilizer-gradient-flow {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.stabilizer-equations {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(190px, 0.6fr);
  align-items: center;
  gap: 8px;
}

.stabilizer-equations code,
.stabilizer-equations span {
  min-width: 0;
  border-radius: 8px;
  padding: 10px;
}

.stabilizer-equations code {
  background: rgba(37, 99, 235, 0.07);
  color: #234c9f;
  overflow-wrap: anywhere;
}

.stabilizer-equations span {
  color: $muted;
  font-size: 0.75rem;
}

.stabilizer-audit-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.stabilizer-audit-grid > div {
  display: grid;
  gap: 6px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 8px;
  padding: 10px;
}

.stabilizer-audit-grid code {
  color: #1d6849;
}

.stabilizer-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 10px;
  padding: 16px;
}

.stabilizer-route-buttons {
  display: grid;
  gap: 7px;
}

.stabilizer-route-buttons button {
  display: grid;
  gap: 4px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.stabilizer-route-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.48);
  background: rgba(109, 91, 208, 0.1);
  color: #4b3e9d;
}

.stabilizer-route-buttons span {
  color: $muted;
  font-size: 0.7rem;
}

.stabilizer-coordinate-buttons {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 6px;
}

.stabilizer-coordinate-buttons button {
  display: grid;
  gap: 3px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 7px 4px;
}

.stabilizer-coordinate-buttons button[aria-pressed="true"] {
  border-color: rgba(194, 65, 59, 0.5);
  background: rgba(194, 65, 59, 0.075);
}

.stabilizer-coordinate-buttons code {
  color: $muted;
  font-size: 0.62rem;
}

.stabilizer-reading {
  display: grid;
  gap: 7px;
  margin-top: 6px;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.065);
  padding: 11px;
}

.stabilizer-reading h2,
.stabilizer-reading p {
  margin: 0;
}

@media (max-width: 1180px) {
  .workspace--stabilizers {
    grid-template-columns: 1fr;
  }

  .stabilizer-controls {
    position: static;
  }

  .stabilizer-route-buttons {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .stabilizer-intro,
  .stabilizer-common-panel .panel-heading,
  .stabilizer-comparison-panel .panel-heading,
  .stabilizer-forward-panel .panel-heading,
  .stabilizer-backward-panel .panel-heading,
  .stabilizer-arithmetic-panel .panel-heading,
  .stabilizer-audit-panel .panel-heading {
    display: grid;
  }

  .stabilizer-chip {
    justify-self: start;
  }

  .stabilizer-common-flow,
  .stabilizer-comparison-grid,
  .stabilizer-stat-grid,
  .stabilizer-dropout-compare,
  .stabilizer-gradient-flow,
  .stabilizer-equations,
  .stabilizer-audit-grid,
  .stabilizer-route-buttons {
    grid-template-columns: 1fr;
  }

  .stabilizer-flow-arrow {
    transform: rotate(90deg);
  }
}

.workspace--gradient-buffer {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.gradient-buffer-stage {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.gradient-buffer-intro,
.gradient-buffer-state,
.gradient-buffer-timeline,
.gradient-buffer-equation,
.gradient-buffer-audit,
.gradient-buffer-controls {
  @include surface;
}

.gradient-buffer-intro,
.gradient-buffer-state,
.gradient-buffer-timeline,
.gradient-buffer-equation,
.gradient-buffer-audit {
  padding: 18px;
}

.gradient-buffer-intro {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.gradient-buffer-intro h2,
.gradient-buffer-controls h2 {
  margin: 0;
}

.gradient-buffer-intro p:not(.eyebrow),
.gradient-buffer-controls > p {
  color: $muted;
  line-height: 1.55;
}

.gradient-buffer-chip {
  flex: 0 0 auto;
  border: 1px solid rgba(109, 91, 208, 0.38);
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.08);
  color: #4b3e9d;
  padding: 8px 12px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 0.75rem;
  font-weight: 850;
}

.gradient-buffer-state,
.gradient-buffer-timeline,
.gradient-buffer-equation,
.gradient-buffer-audit {
  display: grid;
  gap: 14px;
}

.gradient-buffer-vessels {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.gradient-buffer-vessels > div {
  display: grid;
  grid-template-columns: 1fr auto auto auto;
  align-items: center;
  gap: 10px;
  min-width: 0;
  border: 1px solid rgba(37, 99, 235, 0.2);
  border-radius: 10px;
  background: rgba(37, 99, 235, 0.055);
  padding: 13px;
}

.gradient-buffer-vessels > div.is-filled {
  border-color: rgba(194, 65, 59, 0.35);
  background: linear-gradient(0deg, rgba(194, 65, 59, 0.11), rgba(255, 255, 255, 0.75));
}

.gradient-buffer-vessels > div.is-empty {
  border-color: rgba(35, 122, 87, 0.3);
  background: rgba(35, 122, 87, 0.055);
}

.gradient-buffer-vessels small,
.gradient-buffer-backward-grid small,
.gradient-buffer-step-rule small,
.gradient-buffer-audit-grid small {
  color: $muted;
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.gradient-buffer-vessels code,
.gradient-buffer-vessels strong,
.gradient-buffer-event-lane code,
.gradient-buffer-backward-grid code,
.gradient-buffer-backward-grid strong,
.gradient-buffer-zero-rule code,
.gradient-buffer-step-rule code,
.gradient-buffer-step-rule strong,
.gradient-buffer-audit-grid code,
.gradient-buffer-summary code {
  font-family: "SFMono-Regular", Consolas, monospace;
}

.gradient-buffer-vessels span {
  color: $muted;
  font-weight: 900;
}

.gradient-buffer-vessels strong {
  color: #1d6849;
  font-size: 1.12rem;
}

.gradient-buffer-event-lane {
  display: grid;
  grid-template-columns: repeat(5, minmax(150px, 1fr));
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.gradient-buffer-event-lane button {
  display: grid;
  gap: 6px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.82);
  color: $ink;
  padding: 11px;
  text-align: left;
}

.gradient-buffer-event-lane button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.52);
  background: rgba(109, 91, 208, 0.09);
  box-shadow: inset 0 -3px rgba(109, 91, 208, 0.14);
}

.gradient-buffer-event-lane small,
.gradient-buffer-event-lane code {
  color: $muted;
  font-size: 0.67rem;
}

.gradient-buffer-event-lane strong {
  overflow-wrap: anywhere;
}

.gradient-buffer-backward-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 9px;
}

.gradient-buffer-backward-grid > div,
.gradient-buffer-step-rule > div,
.gradient-buffer-audit-grid > div {
  display: grid;
  gap: 7px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.045);
  padding: 11px;
}

.gradient-buffer-backward-grid code,
.gradient-buffer-step-rule code {
  color: #234c9f;
  overflow-wrap: anywhere;
}

.gradient-buffer-backward-grid strong,
.gradient-buffer-step-rule strong {
  color: #1d6849;
}

.gradient-buffer-backward-grid > div.gradient-buffer-addition {
  border-color: rgba(194, 65, 59, 0.25);
  background: rgba(194, 65, 59, 0.055);
}

.gradient-buffer-zero-rule {
  display: grid;
  grid-template-columns: minmax(170px, 0.4fr) minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.06);
  padding: 14px;
}

.gradient-buffer-zero-rule code {
  color: #1d6849;
  font-size: 1.05rem;
}

.gradient-buffer-zero-rule p,
.gradient-buffer-step-rule p {
  margin: 0;
  color: $muted;
  line-height: 1.5;
}

.gradient-buffer-step-rule {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 9px;
}

.gradient-buffer-step-rule p {
  grid-column: 1 / -1;
  border-radius: 8px;
  background: rgba(183, 121, 31, 0.08);
  color: #7a5318;
  padding: 10px;
}

.gradient-buffer-audit-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.gradient-buffer-audit-grid span {
  color: $muted;
  font-size: 0.74rem;
}

.gradient-buffer-audit-grid code {
  color: #1d6849;
}

.gradient-buffer-audit-grid > div.gradient-buffer-audit-max {
  border-color: rgba(35, 122, 87, 0.3);
  background: rgba(35, 122, 87, 0.065);
}

.gradient-buffer-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 11px;
  padding: 16px;
}

.gradient-buffer-scenario-buttons {
  display: grid;
  gap: 7px;
}

.gradient-buffer-scenario-buttons button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 10px;
  text-align: left;
}

.gradient-buffer-scenario-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.09);
}

.gradient-buffer-scenario-buttons span {
  color: $muted;
  font-size: 0.7rem;
  line-height: 1.35;
}

.gradient-buffer-summary,
.gradient-buffer-mental-model {
  display: grid;
  gap: 7px;
  margin-top: 5px;
  border-radius: 9px;
  padding: 11px;
}

.gradient-buffer-summary {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  background: rgba(37, 99, 235, 0.055);
}

.gradient-buffer-summary .eyebrow {
  grid-column: 1 / -1;
}

.gradient-buffer-summary code {
  color: #234c9f;
}

.gradient-buffer-mental-model {
  background: rgba(35, 122, 87, 0.065);
}

.gradient-buffer-mental-model h2,
.gradient-buffer-mental-model p {
  margin: 0;
}

.gradient-buffer-mental-model p:not(.eyebrow) {
  color: $muted;
  line-height: 1.45;
}

@media (max-width: 1180px) {
  .workspace--gradient-buffer {
    grid-template-columns: 1fr;
  }

  .gradient-buffer-controls {
    position: static;
  }

  .gradient-buffer-scenario-buttons {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 820px) {
  .gradient-buffer-intro,
  .gradient-buffer-state .panel-heading,
  .gradient-buffer-timeline .panel-heading,
  .gradient-buffer-equation .panel-heading,
  .gradient-buffer-audit .panel-heading {
    display: grid;
  }

  .gradient-buffer-chip {
    justify-self: start;
  }

  .gradient-buffer-vessels,
  .gradient-buffer-backward-grid,
  .gradient-buffer-zero-rule,
  .gradient-buffer-step-rule,
  .gradient-buffer-audit-grid,
  .gradient-buffer-scenario-buttons {
    grid-template-columns: 1fr;
  }

  .gradient-buffer-step-rule p,
  .gradient-buffer-summary .eyebrow {
    grid-column: auto;
  }
}
`,Hp=`coding-adventures-lattice-styles`;function Up(){if(document.getElementById(Hp)===null)try{let e=document.createElement(`style`);e.id=Hp,e.textContent=Bp(Vp),document.head.append(e)}catch(e){console.error(`Failed to install Lattice styles`,e)}}Up(),(0,u.createRoot)(document.getElementById(`root`)).render((0,T.jsx)(l.StrictMode,{children:(0,T.jsx)(Ad,{})}));