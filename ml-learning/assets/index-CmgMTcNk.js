var e=(e,t)=>()=>(t||(e((t={exports:{}}).exports,t),e=null),t.exports);(function(){let e=document.createElement(`link`).relList;if(e&&e.supports&&e.supports(`modulepreload`))return;for(let e of document.querySelectorAll(`link[rel="modulepreload"]`))n(e);new MutationObserver(e=>{for(let t of e)if(t.type===`childList`)for(let e of t.addedNodes)e.tagName===`LINK`&&e.rel===`modulepreload`&&n(e)}).observe(document,{childList:!0,subtree:!0});function t(e){let t={};return e.integrity&&(t.integrity=e.integrity),e.referrerPolicy&&(t.referrerPolicy=e.referrerPolicy),e.crossOrigin===`use-credentials`?t.credentials=`include`:e.crossOrigin===`anonymous`?t.credentials=`omit`:t.credentials=`same-origin`,t}function n(e){if(e.ep)return;e.ep=!0;let n=t(e);fetch(e.href,n)}})();var t=e((e=>{var t=Symbol.for(`react.transitional.element`),n=Symbol.for(`react.portal`),r=Symbol.for(`react.fragment`),i=Symbol.for(`react.strict_mode`),a=Symbol.for(`react.profiler`),o=Symbol.for(`react.consumer`),s=Symbol.for(`react.context`),c=Symbol.for(`react.forward_ref`),l=Symbol.for(`react.suspense`),u=Symbol.for(`react.memo`),d=Symbol.for(`react.lazy`),f=Symbol.for(`react.activity`),p=Symbol.iterator;function m(e){return typeof e!=`object`||!e?null:(e=p&&e[p]||e[`@@iterator`],typeof e==`function`?e:null)}var h={isMounted:function(){return!1},enqueueForceUpdate:function(){},enqueueReplaceState:function(){},enqueueSetState:function(){}},g=Object.assign,_={};function v(e,t,n){this.props=e,this.context=t,this.refs=_,this.updater=n||h}v.prototype.isReactComponent={},v.prototype.setState=function(e,t){if(typeof e!=`object`&&typeof e!=`function`&&e!=null)throw Error(`takes an object of state variables to update or a function which returns an object of state variables.`);this.updater.enqueueSetState(this,e,t,`setState`)},v.prototype.forceUpdate=function(e){this.updater.enqueueForceUpdate(this,e,`forceUpdate`)};function y(){}y.prototype=v.prototype;function b(e,t,n){this.props=e,this.context=t,this.refs=_,this.updater=n||h}var x=b.prototype=new y;x.constructor=b,g(x,v.prototype),x.isPureReactComponent=!0;var S=Array.isArray;function C(){}var w={H:null,A:null,T:null,S:null},ee=Object.prototype.hasOwnProperty;function te(e,n,r){var i=r.ref;return{$$typeof:t,type:e,key:n,ref:i===void 0?null:i,props:r}}function T(e,t){return te(e.type,t,e.props)}function ne(e){return typeof e==`object`&&!!e&&e.$$typeof===t}function re(e){var t={"=":`=0`,":":`=2`};return`$`+e.replace(/[=:]/g,function(e){return t[e]})}var ie=/\/+/g;function E(e,t){return typeof e==`object`&&e&&e.key!=null?re(``+e.key):t.toString(36)}function ae(e){switch(e.status){case`fulfilled`:return e.value;case`rejected`:throw e.reason;default:switch(typeof e.status==`string`?e.then(C,C):(e.status=`pending`,e.then(function(t){e.status===`pending`&&(e.status=`fulfilled`,e.value=t)},function(t){e.status===`pending`&&(e.status=`rejected`,e.reason=t)})),e.status){case`fulfilled`:return e.value;case`rejected`:throw e.reason}}throw e}function oe(e,r,i,a,o){var s=typeof e;(s===`undefined`||s===`boolean`)&&(e=null);var c=!1;if(e===null)c=!0;else switch(s){case`bigint`:case`string`:case`number`:c=!0;break;case`object`:switch(e.$$typeof){case t:case n:c=!0;break;case d:return c=e._init,oe(c(e._payload),r,i,a,o)}}if(c)return o=o(e),c=a===``?`.`+E(e,0):a,S(o)?(i=``,c!=null&&(i=c.replace(ie,`$&/`)+`/`),oe(o,r,i,``,function(e){return e})):o!=null&&(ne(o)&&(o=T(o,i+(o.key==null||e&&e.key===o.key?``:(``+o.key).replace(ie,`$&/`)+`/`)+c)),r.push(o)),1;c=0;var l=a===``?`.`:a+`:`;if(S(e))for(var u=0;u<e.length;u++)a=e[u],s=l+E(a,u),c+=oe(a,r,i,s,o);else if(u=m(e),typeof u==`function`)for(e=u.call(e),u=0;!(a=e.next()).done;)a=a.value,s=l+E(a,u++),c+=oe(a,r,i,s,o);else if(s===`object`){if(typeof e.then==`function`)return oe(ae(e),r,i,a,o);throw r=String(e),Error(`Objects are not valid as a React child (found: `+(r===`[object Object]`?`object with keys {`+Object.keys(e).join(`, `)+`}`:r)+`). If you meant to render a collection of children, use an array instead.`)}return c}function se(e,t,n){if(e==null)return e;var r=[],i=0;return oe(e,r,``,``,function(e){return t.call(n,e,i++)}),r}function ce(e){if(e._status===-1){var t=e._result;t=t(),t.then(function(t){(e._status===0||e._status===-1)&&(e._status=1,e._result=t)},function(t){(e._status===0||e._status===-1)&&(e._status=2,e._result=t)}),e._status===-1&&(e._status=0,e._result=t)}if(e._status===1)return e._result.default;throw e._result}var D=typeof reportError==`function`?reportError:function(e){if(typeof window==`object`&&typeof window.ErrorEvent==`function`){var t=new window.ErrorEvent(`error`,{bubbles:!0,cancelable:!0,message:typeof e==`object`&&e&&typeof e.message==`string`?String(e.message):String(e),error:e});if(!window.dispatchEvent(t))return}else if(typeof process==`object`&&typeof process.emit==`function`){process.emit(`uncaughtException`,e);return}console.error(e)},O={map:se,forEach:function(e,t,n){se(e,function(){t.apply(this,arguments)},n)},count:function(e){var t=0;return se(e,function(){t++}),t},toArray:function(e){return se(e,function(e){return e})||[]},only:function(e){if(!ne(e))throw Error(`React.Children.only expected to receive a single React element child.`);return e}};e.Activity=f,e.Children=O,e.Component=v,e.Fragment=r,e.Profiler=a,e.PureComponent=b,e.StrictMode=i,e.Suspense=l,e.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=w,e.__COMPILER_RUNTIME={__proto__:null,c:function(e){return w.H.useMemoCache(e)}},e.cache=function(e){return function(){return e.apply(null,arguments)}},e.cacheSignal=function(){return null},e.cloneElement=function(e,t,n){if(e==null)throw Error(`The argument must be a React element, but you passed `+e+`.`);var r=g({},e.props),i=e.key;if(t!=null)for(a in t.key!==void 0&&(i=``+t.key),t)!ee.call(t,a)||a===`key`||a===`__self`||a===`__source`||a===`ref`&&t.ref===void 0||(r[a]=t[a]);var a=arguments.length-2;if(a===1)r.children=n;else if(1<a){for(var o=Array(a),s=0;s<a;s++)o[s]=arguments[s+2];r.children=o}return te(e.type,i,r)},e.createContext=function(e){return e={$$typeof:s,_currentValue:e,_currentValue2:e,_threadCount:0,Provider:null,Consumer:null},e.Provider=e,e.Consumer={$$typeof:o,_context:e},e},e.createElement=function(e,t,n){var r,i={},a=null;if(t!=null)for(r in t.key!==void 0&&(a=``+t.key),t)ee.call(t,r)&&r!==`key`&&r!==`__self`&&r!==`__source`&&(i[r]=t[r]);var o=arguments.length-2;if(o===1)i.children=n;else if(1<o){for(var s=Array(o),c=0;c<o;c++)s[c]=arguments[c+2];i.children=s}if(e&&e.defaultProps)for(r in o=e.defaultProps,o)i[r]===void 0&&(i[r]=o[r]);return te(e,a,i)},e.createRef=function(){return{current:null}},e.forwardRef=function(e){return{$$typeof:c,render:e}},e.isValidElement=ne,e.lazy=function(e){return{$$typeof:d,_payload:{_status:-1,_result:e},_init:ce}},e.memo=function(e,t){return{$$typeof:u,type:e,compare:t===void 0?null:t}},e.startTransition=function(e){var t=w.T,n={};w.T=n;try{var r=e(),i=w.S;i!==null&&i(n,r),typeof r==`object`&&r&&typeof r.then==`function`&&r.then(C,D)}catch(e){D(e)}finally{t!==null&&n.types!==null&&(t.types=n.types),w.T=t}},e.unstable_useCacheRefresh=function(){return w.H.useCacheRefresh()},e.use=function(e){return w.H.use(e)},e.useActionState=function(e,t,n){return w.H.useActionState(e,t,n)},e.useCallback=function(e,t){return w.H.useCallback(e,t)},e.useContext=function(e){return w.H.useContext(e)},e.useDebugValue=function(){},e.useDeferredValue=function(e,t){return w.H.useDeferredValue(e,t)},e.useEffect=function(e,t){return w.H.useEffect(e,t)},e.useEffectEvent=function(e){return w.H.useEffectEvent(e)},e.useId=function(){return w.H.useId()},e.useImperativeHandle=function(e,t,n){return w.H.useImperativeHandle(e,t,n)},e.useInsertionEffect=function(e,t){return w.H.useInsertionEffect(e,t)},e.useLayoutEffect=function(e,t){return w.H.useLayoutEffect(e,t)},e.useMemo=function(e,t){return w.H.useMemo(e,t)},e.useOptimistic=function(e,t){return w.H.useOptimistic(e,t)},e.useReducer=function(e,t,n){return w.H.useReducer(e,t,n)},e.useRef=function(e){return w.H.useRef(e)},e.useState=function(e){return w.H.useState(e)},e.useSyncExternalStore=function(e,t,n){return w.H.useSyncExternalStore(e,t,n)},e.useTransition=function(){return w.H.useTransition()},e.version=`19.2.7`})),n=e(((e,n)=>{n.exports=t()})),r=e((e=>{function t(e,t){var n=e.length;e.push(t);a:for(;0<n;){var r=n-1>>>1,a=e[r];if(0<i(a,t))e[r]=t,e[n]=a,n=r;else break a}}function n(e){return e.length===0?null:e[0]}function r(e){if(e.length===0)return null;var t=e[0],n=e.pop();if(n!==t){e[0]=n;a:for(var r=0,a=e.length,o=a>>>1;r<o;){var s=2*(r+1)-1,c=e[s],l=s+1,u=e[l];if(0>i(c,n))l<a&&0>i(u,c)?(e[r]=u,e[l]=n,r=l):(e[r]=c,e[s]=n,r=s);else if(l<a&&0>i(u,n))e[r]=u,e[l]=n,r=l;else break a}}return t}function i(e,t){var n=e.sortIndex-t.sortIndex;return n===0?e.id-t.id:n}if(e.unstable_now=void 0,typeof performance==`object`&&typeof performance.now==`function`){var a=performance;e.unstable_now=function(){return a.now()}}else{var o=Date,s=o.now();e.unstable_now=function(){return o.now()-s}}var c=[],l=[],u=1,d=null,f=3,p=!1,m=!1,h=!1,g=!1,_=typeof setTimeout==`function`?setTimeout:null,v=typeof clearTimeout==`function`?clearTimeout:null,y=typeof setImmediate<`u`?setImmediate:null;function b(e){for(var i=n(l);i!==null;){if(i.callback===null)r(l);else if(i.startTime<=e)r(l),i.sortIndex=i.expirationTime,t(c,i);else break;i=n(l)}}function x(e){if(h=!1,b(e),!m)if(n(c)!==null)m=!0,S||(S=!0,ne());else{var t=n(l);t!==null&&E(x,t.startTime-e)}}var S=!1,C=-1,w=5,ee=-1;function te(){return g?!0:!(e.unstable_now()-ee<w)}function T(){if(g=!1,S){var t=e.unstable_now();ee=t;var i=!0;try{a:{m=!1,h&&(h=!1,v(C),C=-1),p=!0;var a=f;try{b:{for(b(t),d=n(c);d!==null&&!(d.expirationTime>t&&te());){var o=d.callback;if(typeof o==`function`){d.callback=null,f=d.priorityLevel;var s=o(d.expirationTime<=t);if(t=e.unstable_now(),typeof s==`function`){d.callback=s,b(t),i=!0;break b}d===n(c)&&r(c),b(t)}else r(c);d=n(c)}if(d!==null)i=!0;else{var u=n(l);u!==null&&E(x,u.startTime-t),i=!1}}break a}finally{d=null,f=a,p=!1}i=void 0}}finally{i?ne():S=!1}}}var ne;if(typeof y==`function`)ne=function(){y(T)};else if(typeof MessageChannel<`u`){var re=new MessageChannel,ie=re.port2;re.port1.onmessage=T,ne=function(){ie.postMessage(null)}}else ne=function(){_(T,0)};function E(t,n){C=_(function(){t(e.unstable_now())},n)}e.unstable_IdlePriority=5,e.unstable_ImmediatePriority=1,e.unstable_LowPriority=4,e.unstable_NormalPriority=3,e.unstable_Profiling=null,e.unstable_UserBlockingPriority=2,e.unstable_cancelCallback=function(e){e.callback=null},e.unstable_forceFrameRate=function(e){0>e||125<e?console.error(`forceFrameRate takes a positive int between 0 and 125, forcing frame rates higher than 125 fps is not supported`):w=0<e?Math.floor(1e3/e):5},e.unstable_getCurrentPriorityLevel=function(){return f},e.unstable_next=function(e){switch(f){case 1:case 2:case 3:var t=3;break;default:t=f}var n=f;f=t;try{return e()}finally{f=n}},e.unstable_requestPaint=function(){g=!0},e.unstable_runWithPriority=function(e,t){switch(e){case 1:case 2:case 3:case 4:case 5:break;default:e=3}var n=f;f=e;try{return t()}finally{f=n}},e.unstable_scheduleCallback=function(r,i,a){var o=e.unstable_now();switch(typeof a==`object`&&a?(a=a.delay,a=typeof a==`number`&&0<a?o+a:o):a=o,r){case 1:var s=-1;break;case 2:s=250;break;case 5:s=1073741823;break;case 4:s=1e4;break;default:s=5e3}return s=a+s,r={id:u++,callback:i,priorityLevel:r,startTime:a,expirationTime:s,sortIndex:-1},a>o?(r.sortIndex=a,t(l,r),n(c)===null&&r===n(l)&&(h?(v(C),C=-1):h=!0,E(x,a-o))):(r.sortIndex=s,t(c,r),m||p||(m=!0,S||(S=!0,ne()))),r},e.unstable_shouldYield=te,e.unstable_wrapCallback=function(e){var t=f;return function(){var n=f;f=t;try{return e.apply(this,arguments)}finally{f=n}}}})),i=e(((e,t)=>{t.exports=r()})),a=e((e=>{var t=n();function r(e){var t=`https://react.dev/errors/`+e;if(1<arguments.length){t+=`?args[]=`+encodeURIComponent(arguments[1]);for(var n=2;n<arguments.length;n++)t+=`&args[]=`+encodeURIComponent(arguments[n])}return`Minified React error #`+e+`; visit `+t+` for the full message or use the non-minified dev environment for full errors and additional helpful warnings.`}function i(){}var a={d:{f:i,r:function(){throw Error(r(522))},D:i,C:i,L:i,m:i,X:i,S:i,M:i},p:0,findDOMNode:null},o=Symbol.for(`react.portal`);function s(e,t,n){var r=3<arguments.length&&arguments[3]!==void 0?arguments[3]:null;return{$$typeof:o,key:r==null?null:``+r,children:e,containerInfo:t,implementation:n}}var c=t.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;function l(e,t){if(e===`font`)return``;if(typeof t==`string`)return t===`use-credentials`?t:``}e.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=a,e.createPortal=function(e,t){var n=2<arguments.length&&arguments[2]!==void 0?arguments[2]:null;if(!t||t.nodeType!==1&&t.nodeType!==9&&t.nodeType!==11)throw Error(r(299));return s(e,t,null,n)},e.flushSync=function(e){var t=c.T,n=a.p;try{if(c.T=null,a.p=2,e)return e()}finally{c.T=t,a.p=n,a.d.f()}},e.preconnect=function(e,t){typeof e==`string`&&(t?(t=t.crossOrigin,t=typeof t==`string`?t===`use-credentials`?t:``:void 0):t=null,a.d.C(e,t))},e.prefetchDNS=function(e){typeof e==`string`&&a.d.D(e)},e.preinit=function(e,t){if(typeof e==`string`&&t&&typeof t.as==`string`){var n=t.as,r=l(n,t.crossOrigin),i=typeof t.integrity==`string`?t.integrity:void 0,o=typeof t.fetchPriority==`string`?t.fetchPriority:void 0;n===`style`?a.d.S(e,typeof t.precedence==`string`?t.precedence:void 0,{crossOrigin:r,integrity:i,fetchPriority:o}):n===`script`&&a.d.X(e,{crossOrigin:r,integrity:i,fetchPriority:o,nonce:typeof t.nonce==`string`?t.nonce:void 0})}},e.preinitModule=function(e,t){if(typeof e==`string`)if(typeof t==`object`&&t){if(t.as==null||t.as===`script`){var n=l(t.as,t.crossOrigin);a.d.M(e,{crossOrigin:n,integrity:typeof t.integrity==`string`?t.integrity:void 0,nonce:typeof t.nonce==`string`?t.nonce:void 0})}}else t??a.d.M(e)},e.preload=function(e,t){if(typeof e==`string`&&typeof t==`object`&&t&&typeof t.as==`string`){var n=t.as,r=l(n,t.crossOrigin);a.d.L(e,n,{crossOrigin:r,integrity:typeof t.integrity==`string`?t.integrity:void 0,nonce:typeof t.nonce==`string`?t.nonce:void 0,type:typeof t.type==`string`?t.type:void 0,fetchPriority:typeof t.fetchPriority==`string`?t.fetchPriority:void 0,referrerPolicy:typeof t.referrerPolicy==`string`?t.referrerPolicy:void 0,imageSrcSet:typeof t.imageSrcSet==`string`?t.imageSrcSet:void 0,imageSizes:typeof t.imageSizes==`string`?t.imageSizes:void 0,media:typeof t.media==`string`?t.media:void 0})}},e.preloadModule=function(e,t){if(typeof e==`string`)if(t){var n=l(t.as,t.crossOrigin);a.d.m(e,{as:typeof t.as==`string`&&t.as!==`script`?t.as:void 0,crossOrigin:n,integrity:typeof t.integrity==`string`?t.integrity:void 0})}else a.d.m(e)},e.requestFormReset=function(e){a.d.r(e)},e.unstable_batchedUpdates=function(e,t){return e(t)},e.useFormState=function(e,t,n){return c.H.useFormState(e,t,n)},e.useFormStatus=function(){return c.H.useHostTransitionStatus()},e.version=`19.2.7`})),o=e(((e,t)=>{function n(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>`u`||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!=`function`))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(n)}catch(e){console.error(e)}}n(),t.exports=a()})),s=e((e=>{var t=i(),r=n(),a=o();function s(e){var t=`https://react.dev/errors/`+e;if(1<arguments.length){t+=`?args[]=`+encodeURIComponent(arguments[1]);for(var n=2;n<arguments.length;n++)t+=`&args[]=`+encodeURIComponent(arguments[n])}return`Minified React error #`+e+`; visit `+t+` for the full message or use the non-minified dev environment for full errors and additional helpful warnings.`}function c(e){return!(!e||e.nodeType!==1&&e.nodeType!==9&&e.nodeType!==11)}function l(e){var t=e,n=e;if(e.alternate)for(;t.return;)t=t.return;else{e=t;do t=e,t.flags&4098&&(n=t.return),e=t.return;while(e)}return t.tag===3?n:null}function u(e){if(e.tag===13){var t=e.memoizedState;if(t===null&&(e=e.alternate,e!==null&&(t=e.memoizedState)),t!==null)return t.dehydrated}return null}function d(e){if(e.tag===31){var t=e.memoizedState;if(t===null&&(e=e.alternate,e!==null&&(t=e.memoizedState)),t!==null)return t.dehydrated}return null}function f(e){if(l(e)!==e)throw Error(s(188))}function p(e){var t=e.alternate;if(!t){if(t=l(e),t===null)throw Error(s(188));return t===e?e:null}for(var n=e,r=t;;){var i=n.return;if(i===null)break;var a=i.alternate;if(a===null){if(r=i.return,r!==null){n=r;continue}break}if(i.child===a.child){for(a=i.child;a;){if(a===n)return f(i),e;if(a===r)return f(i),t;a=a.sibling}throw Error(s(188))}if(n.return!==r.return)n=i,r=a;else{for(var o=!1,c=i.child;c;){if(c===n){o=!0,n=i,r=a;break}if(c===r){o=!0,r=i,n=a;break}c=c.sibling}if(!o){for(c=a.child;c;){if(c===n){o=!0,n=a,r=i;break}if(c===r){o=!0,r=a,n=i;break}c=c.sibling}if(!o)throw Error(s(189))}}if(n.alternate!==r)throw Error(s(190))}if(n.tag!==3)throw Error(s(188));return n.stateNode.current===n?e:t}function m(e){var t=e.tag;if(t===5||t===26||t===27||t===6)return e;for(e=e.child;e!==null;){if(t=m(e),t!==null)return t;e=e.sibling}return null}var h=Object.assign,g=Symbol.for(`react.element`),_=Symbol.for(`react.transitional.element`),v=Symbol.for(`react.portal`),y=Symbol.for(`react.fragment`),b=Symbol.for(`react.strict_mode`),x=Symbol.for(`react.profiler`),S=Symbol.for(`react.consumer`),C=Symbol.for(`react.context`),w=Symbol.for(`react.forward_ref`),ee=Symbol.for(`react.suspense`),te=Symbol.for(`react.suspense_list`),T=Symbol.for(`react.memo`),ne=Symbol.for(`react.lazy`),re=Symbol.for(`react.activity`),ie=Symbol.for(`react.memo_cache_sentinel`),E=Symbol.iterator;function ae(e){return typeof e!=`object`||!e?null:(e=E&&e[E]||e[`@@iterator`],typeof e==`function`?e:null)}var oe=Symbol.for(`react.client.reference`);function se(e){if(e==null)return null;if(typeof e==`function`)return e.$$typeof===oe?null:e.displayName||e.name||null;if(typeof e==`string`)return e;switch(e){case y:return`Fragment`;case x:return`Profiler`;case b:return`StrictMode`;case ee:return`Suspense`;case te:return`SuspenseList`;case re:return`Activity`}if(typeof e==`object`)switch(e.$$typeof){case v:return`Portal`;case C:return e.displayName||`Context`;case S:return(e._context.displayName||`Context`)+`.Consumer`;case w:var t=e.render;return e=e.displayName,e||=(e=t.displayName||t.name||``,e===``?`ForwardRef`:`ForwardRef(`+e+`)`),e;case T:return t=e.displayName||null,t===null?se(e.type)||`Memo`:t;case ne:t=e._payload,e=e._init;try{return se(e(t))}catch{}}return null}var ce=Array.isArray,D=r.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,O=a.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,le={pending:!1,data:null,method:null,action:null},ue=[],de=-1;function fe(e){return{current:e}}function pe(e){0>de||(e.current=ue[de],ue[de]=null,de--)}function me(e,t){de++,ue[de]=e.current,e.current=t}var he=fe(null),ge=fe(null),_e=fe(null),ve=fe(null);function ye(e,t){switch(me(_e,t),me(ge,e),me(he,null),t.nodeType){case 9:case 11:e=(e=t.documentElement)&&(e=e.namespaceURI)?Wd(e):0;break;default:if(e=t.tagName,t=t.namespaceURI)t=Wd(t),e=Gd(t,e);else switch(e){case`svg`:e=1;break;case`math`:e=2;break;default:e=0}}pe(he),me(he,e)}function be(){pe(he),pe(ge),pe(_e)}function xe(e){e.memoizedState!==null&&me(ve,e);var t=he.current,n=Gd(t,e.type);t!==n&&(me(ge,e),me(he,n))}function Se(e){ge.current===e&&(pe(he),pe(ge)),ve.current===e&&(pe(ve),ep._currentValue=le)}var Ce,we;function Te(e){if(Ce===void 0)try{throw Error()}catch(e){var t=e.stack.trim().match(/\n( *(at )?)/);Ce=t&&t[1]||``,we=-1<e.stack.indexOf(`
    at`)?` (<anonymous>)`:-1<e.stack.indexOf(`@`)?`@unknown:0:0`:``}return`
`+Ce+e+we}var Ee=!1;function De(e,t){if(!e||Ee)return``;Ee=!0;var n=Error.prepareStackTrace;Error.prepareStackTrace=void 0;try{var r={DetermineComponentFrameRoot:function(){try{if(t){var n=function(){throw Error()};if(Object.defineProperty(n.prototype,"props",{set:function(){throw Error()}}),typeof Reflect==`object`&&Reflect.construct){try{Reflect.construct(n,[])}catch(e){var r=e}Reflect.construct(e,[],n)}else{try{n.call()}catch(e){r=e}e.call(n.prototype)}}else{try{throw Error()}catch(e){r=e}(n=e())&&typeof n.catch==`function`&&n.catch(function(){})}}catch(e){if(e&&r&&typeof e.stack==`string`)return[e.stack,r.stack]}return[null,null]}};r.DetermineComponentFrameRoot.displayName=`DetermineComponentFrameRoot`;var i=Object.getOwnPropertyDescriptor(r.DetermineComponentFrameRoot,`name`);i&&i.configurable&&Object.defineProperty(r.DetermineComponentFrameRoot,"name",{value:`DetermineComponentFrameRoot`});var a=r.DetermineComponentFrameRoot(),o=a[0],s=a[1];if(o&&s){var c=o.split(`
`),l=s.split(`
`);for(i=r=0;r<c.length&&!c[r].includes(`DetermineComponentFrameRoot`);)r++;for(;i<l.length&&!l[i].includes(`DetermineComponentFrameRoot`);)i++;if(r===c.length||i===l.length)for(r=c.length-1,i=l.length-1;1<=r&&0<=i&&c[r]!==l[i];)i--;for(;1<=r&&0<=i;r--,i--)if(c[r]!==l[i]){if(r!==1||i!==1)do if(r--,i--,0>i||c[r]!==l[i]){var u=`
`+c[r].replace(` at new `,` at `);return e.displayName&&u.includes(`<anonymous>`)&&(u=u.replace(`<anonymous>`,e.displayName)),u}while(1<=r&&0<=i);break}}}finally{Ee=!1,Error.prepareStackTrace=n}return(n=e?e.displayName||e.name:``)?Te(n):``}function Oe(e,t){switch(e.tag){case 26:case 27:case 5:return Te(e.type);case 16:return Te(`Lazy`);case 13:return e.child!==t&&t!==null?Te(`Suspense Fallback`):Te(`Suspense`);case 19:return Te(`SuspenseList`);case 0:case 15:return De(e.type,!1);case 11:return De(e.type.render,!1);case 1:return De(e.type,!0);case 31:return Te(`Activity`);default:return``}}function ke(e){try{var t=``,n=null;do t+=Oe(e,n),n=e,e=e.return;while(e);return t}catch(e){return`
Error generating stack: `+e.message+`
`+e.stack}}var Ae=Object.prototype.hasOwnProperty,je=t.unstable_scheduleCallback,Me=t.unstable_cancelCallback,Ne=t.unstable_shouldYield,Pe=t.unstable_requestPaint,Fe=t.unstable_now,Ie=t.unstable_getCurrentPriorityLevel,Le=t.unstable_ImmediatePriority,Re=t.unstable_UserBlockingPriority,ze=t.unstable_NormalPriority,Be=t.unstable_LowPriority,Ve=t.unstable_IdlePriority,He=t.log,Ue=t.unstable_setDisableYieldValue,We=null,Ge=null;function Ke(e){if(typeof He==`function`&&Ue(e),Ge&&typeof Ge.setStrictMode==`function`)try{Ge.setStrictMode(We,e)}catch{}}var qe=Math.clz32?Math.clz32:Xe,Je=Math.log,Ye=Math.LN2;function Xe(e){return e>>>=0,e===0?32:31-(Je(e)/Ye|0)|0}var Ze=256,Qe=262144,$e=4194304;function et(e){var t=e&42;if(t!==0)return t;switch(e&-e){case 1:return 1;case 2:return 2;case 4:return 4;case 8:return 8;case 16:return 16;case 32:return 32;case 64:return 64;case 128:return 128;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:return e&261888;case 262144:case 524288:case 1048576:case 2097152:return e&3932160;case 4194304:case 8388608:case 16777216:case 33554432:return e&62914560;case 67108864:return 67108864;case 134217728:return 134217728;case 268435456:return 268435456;case 536870912:return 536870912;case 1073741824:return 0;default:return e}}function tt(e,t,n){var r=e.pendingLanes;if(r===0)return 0;var i=0,a=e.suspendedLanes,o=e.pingedLanes;e=e.warmLanes;var s=r&134217727;return s===0?(s=r&~a,s===0?o===0?n||(n=r&~e,n!==0&&(i=et(n))):i=et(o):i=et(s)):(r=s&~a,r===0?(o&=s,o===0?n||(n=s&~e,n!==0&&(i=et(n))):i=et(o)):i=et(r)),i===0?0:t!==0&&t!==i&&(t&a)===0&&(a=i&-i,n=t&-t,a>=n||a===32&&n&4194048)?t:i}function nt(e,t){return(e.pendingLanes&~(e.suspendedLanes&~e.pingedLanes)&t)===0}function rt(e,t){switch(e){case 1:case 2:case 4:case 8:case 64:return t+250;case 16:case 32:case 128:case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:return t+5e3;case 4194304:case 8388608:case 16777216:case 33554432:return-1;case 67108864:case 134217728:case 268435456:case 536870912:case 1073741824:return-1;default:return-1}}function it(){var e=$e;return $e<<=1,!($e&62914560)&&($e=4194304),e}function at(e){for(var t=[],n=0;31>n;n++)t.push(e);return t}function ot(e,t){e.pendingLanes|=t,t!==268435456&&(e.suspendedLanes=0,e.pingedLanes=0,e.warmLanes=0)}function st(e,t,n,r,i,a){var o=e.pendingLanes;e.pendingLanes=n,e.suspendedLanes=0,e.pingedLanes=0,e.warmLanes=0,e.expiredLanes&=n,e.entangledLanes&=n,e.errorRecoveryDisabledLanes&=n,e.shellSuspendCounter=0;var s=e.entanglements,c=e.expirationTimes,l=e.hiddenUpdates;for(n=o&~n;0<n;){var u=31-qe(n),d=1<<u;s[u]=0,c[u]=-1;var f=l[u];if(f!==null)for(l[u]=null,u=0;u<f.length;u++){var p=f[u];p!==null&&(p.lane&=-536870913)}n&=~d}r!==0&&ct(e,r,0),a!==0&&i===0&&e.tag!==0&&(e.suspendedLanes|=a&~(o&~t))}function ct(e,t,n){e.pendingLanes|=t,e.suspendedLanes&=~t;var r=31-qe(t);e.entangledLanes|=t,e.entanglements[r]=e.entanglements[r]|1073741824|n&261930}function lt(e,t){var n=e.entangledLanes|=t;for(e=e.entanglements;n;){var r=31-qe(n),i=1<<r;i&t|e[r]&t&&(e[r]|=t),n&=~i}}function ut(e,t){var n=t&-t;return n=n&42?1:dt(n),(n&(e.suspendedLanes|t))===0?n:0}function dt(e){switch(e){case 2:e=1;break;case 8:e=4;break;case 32:e=16;break;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:case 4194304:case 8388608:case 16777216:case 33554432:e=128;break;case 268435456:e=134217728;break;default:e=0}return e}function ft(e){return e&=-e,2<e?8<e?e&134217727?32:268435456:8:2}function pt(){var e=O.p;return e===0?(e=window.event,e===void 0?32:gp(e.type)):e}function mt(e,t){var n=O.p;try{return O.p=e,t()}finally{O.p=n}}var ht=Math.random().toString(36).slice(2),gt=`__reactFiber$`+ht,_t=`__reactProps$`+ht,vt=`__reactContainer$`+ht,yt=`__reactEvents$`+ht,bt=`__reactListeners$`+ht,xt=`__reactHandles$`+ht,St=`__reactResources$`+ht,Ct=`__reactMarker$`+ht;function wt(e){delete e[gt],delete e[_t],delete e[yt],delete e[bt],delete e[xt]}function Tt(e){var t=e[gt];if(t)return t;for(var n=e.parentNode;n;){if(t=n[vt]||n[gt]){if(n=t.alternate,t.child!==null||n!==null&&n.child!==null)for(e=mf(e);e!==null;){if(n=e[gt])return n;e=mf(e)}return t}e=n,n=e.parentNode}return null}function Et(e){if(e=e[gt]||e[vt]){var t=e.tag;if(t===5||t===6||t===13||t===31||t===26||t===27||t===3)return e}return null}function Dt(e){var t=e.tag;if(t===5||t===26||t===27||t===6)return e.stateNode;throw Error(s(33))}function Ot(e){var t=e[St];return t||=e[St]={hoistableStyles:new Map,hoistableScripts:new Map},t}function kt(e){e[Ct]=!0}var At=new Set,jt={};function Mt(e,t){Nt(e,t),Nt(e+`Capture`,t)}function Nt(e,t){for(jt[e]=t,e=0;e<t.length;e++)At.add(t[e])}var Pt=RegExp(`^[:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD][:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD\\-.0-9\\u00B7\\u0300-\\u036F\\u203F-\\u2040]*$`),Ft={},It={};function Lt(e){return Ae.call(It,e)?!0:Ae.call(Ft,e)?!1:Pt.test(e)?It[e]=!0:(Ft[e]=!0,!1)}function Rt(e,t,n){if(Lt(t))if(n===null)e.removeAttribute(t);else{switch(typeof n){case`undefined`:case`function`:case`symbol`:e.removeAttribute(t);return;case`boolean`:var r=t.toLowerCase().slice(0,5);if(r!==`data-`&&r!==`aria-`){e.removeAttribute(t);return}}e.setAttribute(t,``+n)}}function zt(e,t,n){if(n===null)e.removeAttribute(t);else{switch(typeof n){case`undefined`:case`function`:case`symbol`:case`boolean`:e.removeAttribute(t);return}e.setAttribute(t,``+n)}}function Bt(e,t,n,r){if(r===null)e.removeAttribute(n);else{switch(typeof r){case`undefined`:case`function`:case`symbol`:case`boolean`:e.removeAttribute(n);return}e.setAttributeNS(t,n,``+r)}}function Vt(e){switch(typeof e){case`bigint`:case`boolean`:case`number`:case`string`:case`undefined`:return e;case`object`:return e;default:return``}}function Ht(e){var t=e.type;return(e=e.nodeName)&&e.toLowerCase()===`input`&&(t===`checkbox`||t===`radio`)}function Ut(e,t,n){var r=Object.getOwnPropertyDescriptor(e.constructor.prototype,t);if(!e.hasOwnProperty(t)&&r!==void 0&&typeof r.get==`function`&&typeof r.set==`function`){var i=r.get,a=r.set;return Object.defineProperty(e,t,{configurable:!0,get:function(){return i.call(this)},set:function(e){n=``+e,a.call(this,e)}}),Object.defineProperty(e,t,{enumerable:r.enumerable}),{getValue:function(){return n},setValue:function(e){n=``+e},stopTracking:function(){e._valueTracker=null,delete e[t]}}}}function Wt(e){if(!e._valueTracker){var t=Ht(e)?`checked`:`value`;e._valueTracker=Ut(e,t,``+e[t])}}function Gt(e){if(!e)return!1;var t=e._valueTracker;if(!t)return!0;var n=t.getValue(),r=``;return e&&(r=Ht(e)?e.checked?`true`:`false`:e.value),e=r,e===n?!1:(t.setValue(e),!0)}function Kt(e){if(e||=typeof document<`u`?document:void 0,e===void 0)return null;try{return e.activeElement||e.body}catch{return e.body}}var qt=/[\n"\\]/g;function Jt(e){return e.replace(qt,function(e){return`\\`+e.charCodeAt(0).toString(16)+` `})}function Yt(e,t,n,r,i,a,o,s){e.name=``,o!=null&&typeof o!=`function`&&typeof o!=`symbol`&&typeof o!=`boolean`?e.type=o:e.removeAttribute(`type`),t==null?o!==`submit`&&o!==`reset`||e.removeAttribute(`value`):o===`number`?(t===0&&e.value===``||e.value!=t)&&(e.value=``+Vt(t)):e.value!==``+Vt(t)&&(e.value=``+Vt(t)),t==null?n==null?r!=null&&e.removeAttribute(`value`):Zt(e,o,Vt(n)):Zt(e,o,Vt(t)),i==null&&a!=null&&(e.defaultChecked=!!a),i!=null&&(e.checked=i&&typeof i!=`function`&&typeof i!=`symbol`),s!=null&&typeof s!=`function`&&typeof s!=`symbol`&&typeof s!=`boolean`?e.name=``+Vt(s):e.removeAttribute(`name`)}function Xt(e,t,n,r,i,a,o,s){if(a!=null&&typeof a!=`function`&&typeof a!=`symbol`&&typeof a!=`boolean`&&(e.type=a),t!=null||n!=null){if(!(a!==`submit`&&a!==`reset`||t!=null)){Wt(e);return}n=n==null?``:``+Vt(n),t=t==null?n:``+Vt(t),s||t===e.value||(e.value=t),e.defaultValue=t}r??=i,r=typeof r!=`function`&&typeof r!=`symbol`&&!!r,e.checked=s?e.checked:!!r,e.defaultChecked=!!r,o!=null&&typeof o!=`function`&&typeof o!=`symbol`&&typeof o!=`boolean`&&(e.name=o),Wt(e)}function Zt(e,t,n){t===`number`&&Kt(e.ownerDocument)===e||e.defaultValue===``+n||(e.defaultValue=``+n)}function Qt(e,t,n,r){if(e=e.options,t){t={};for(var i=0;i<n.length;i++)t[`$`+n[i]]=!0;for(n=0;n<e.length;n++)i=t.hasOwnProperty(`$`+e[n].value),e[n].selected!==i&&(e[n].selected=i),i&&r&&(e[n].defaultSelected=!0)}else{for(n=``+Vt(n),t=null,i=0;i<e.length;i++){if(e[i].value===n){e[i].selected=!0,r&&(e[i].defaultSelected=!0);return}t!==null||e[i].disabled||(t=e[i])}t!==null&&(t.selected=!0)}}function $t(e,t,n){if(t!=null&&(t=``+Vt(t),t!==e.value&&(e.value=t),n==null)){e.defaultValue!==t&&(e.defaultValue=t);return}e.defaultValue=n==null?``:``+Vt(n)}function en(e,t,n,r){if(t==null){if(r!=null){if(n!=null)throw Error(s(92));if(ce(r)){if(1<r.length)throw Error(s(93));r=r[0]}n=r}n??=``,t=n}n=Vt(t),e.defaultValue=n,r=e.textContent,r===n&&r!==``&&r!==null&&(e.value=r),Wt(e)}function k(e,t){if(t){var n=e.firstChild;if(n&&n===e.lastChild&&n.nodeType===3){n.nodeValue=t;return}}e.textContent=t}var tn=new Set(`animationIterationCount aspectRatio borderImageOutset borderImageSlice borderImageWidth boxFlex boxFlexGroup boxOrdinalGroup columnCount columns flex flexGrow flexPositive flexShrink flexNegative flexOrder gridArea gridRow gridRowEnd gridRowSpan gridRowStart gridColumn gridColumnEnd gridColumnSpan gridColumnStart fontWeight lineClamp lineHeight opacity order orphans scale tabSize widows zIndex zoom fillOpacity floodOpacity stopOpacity strokeDasharray strokeDashoffset strokeMiterlimit strokeOpacity strokeWidth MozAnimationIterationCount MozBoxFlex MozBoxFlexGroup MozLineClamp msAnimationIterationCount msFlex msZoom msFlexGrow msFlexNegative msFlexOrder msFlexPositive msFlexShrink msGridColumn msGridColumnSpan msGridRow msGridRowSpan WebkitAnimationIterationCount WebkitBoxFlex WebKitBoxFlexGroup WebkitBoxOrdinalGroup WebkitColumnCount WebkitColumns WebkitFlex WebkitFlexGrow WebkitFlexPositive WebkitFlexShrink WebkitLineClamp`.split(` `));function nn(e,t,n){var r=t.indexOf(`--`)===0;n==null||typeof n==`boolean`||n===``?r?e.setProperty(t,``):t===`float`?e.cssFloat=``:e[t]=``:r?e.setProperty(t,n):typeof n!=`number`||n===0||tn.has(t)?t===`float`?e.cssFloat=n:e[t]=(``+n).trim():e[t]=n+`px`}function rn(e,t,n){if(t!=null&&typeof t!=`object`)throw Error(s(62));if(e=e.style,n!=null){for(var r in n)!n.hasOwnProperty(r)||t!=null&&t.hasOwnProperty(r)||(r.indexOf(`--`)===0?e.setProperty(r,``):r===`float`?e.cssFloat=``:e[r]=``);for(var i in t)r=t[i],t.hasOwnProperty(i)&&n[i]!==r&&nn(e,i,r)}else for(var a in t)t.hasOwnProperty(a)&&nn(e,a,t[a])}function an(e){if(e.indexOf(`-`)===-1)return!1;switch(e){case`annotation-xml`:case`color-profile`:case`font-face`:case`font-face-src`:case`font-face-uri`:case`font-face-format`:case`font-face-name`:case`missing-glyph`:return!1;default:return!0}}var on=new Map([[`acceptCharset`,`accept-charset`],[`htmlFor`,`for`],[`httpEquiv`,`http-equiv`],[`crossOrigin`,`crossorigin`],[`accentHeight`,`accent-height`],[`alignmentBaseline`,`alignment-baseline`],[`arabicForm`,`arabic-form`],[`baselineShift`,`baseline-shift`],[`capHeight`,`cap-height`],[`clipPath`,`clip-path`],[`clipRule`,`clip-rule`],[`colorInterpolation`,`color-interpolation`],[`colorInterpolationFilters`,`color-interpolation-filters`],[`colorProfile`,`color-profile`],[`colorRendering`,`color-rendering`],[`dominantBaseline`,`dominant-baseline`],[`enableBackground`,`enable-background`],[`fillOpacity`,`fill-opacity`],[`fillRule`,`fill-rule`],[`floodColor`,`flood-color`],[`floodOpacity`,`flood-opacity`],[`fontFamily`,`font-family`],[`fontSize`,`font-size`],[`fontSizeAdjust`,`font-size-adjust`],[`fontStretch`,`font-stretch`],[`fontStyle`,`font-style`],[`fontVariant`,`font-variant`],[`fontWeight`,`font-weight`],[`glyphName`,`glyph-name`],[`glyphOrientationHorizontal`,`glyph-orientation-horizontal`],[`glyphOrientationVertical`,`glyph-orientation-vertical`],[`horizAdvX`,`horiz-adv-x`],[`horizOriginX`,`horiz-origin-x`],[`imageRendering`,`image-rendering`],[`letterSpacing`,`letter-spacing`],[`lightingColor`,`lighting-color`],[`markerEnd`,`marker-end`],[`markerMid`,`marker-mid`],[`markerStart`,`marker-start`],[`overlinePosition`,`overline-position`],[`overlineThickness`,`overline-thickness`],[`paintOrder`,`paint-order`],[`panose-1`,`panose-1`],[`pointerEvents`,`pointer-events`],[`renderingIntent`,`rendering-intent`],[`shapeRendering`,`shape-rendering`],[`stopColor`,`stop-color`],[`stopOpacity`,`stop-opacity`],[`strikethroughPosition`,`strikethrough-position`],[`strikethroughThickness`,`strikethrough-thickness`],[`strokeDasharray`,`stroke-dasharray`],[`strokeDashoffset`,`stroke-dashoffset`],[`strokeLinecap`,`stroke-linecap`],[`strokeLinejoin`,`stroke-linejoin`],[`strokeMiterlimit`,`stroke-miterlimit`],[`strokeOpacity`,`stroke-opacity`],[`strokeWidth`,`stroke-width`],[`textAnchor`,`text-anchor`],[`textDecoration`,`text-decoration`],[`textRendering`,`text-rendering`],[`transformOrigin`,`transform-origin`],[`underlinePosition`,`underline-position`],[`underlineThickness`,`underline-thickness`],[`unicodeBidi`,`unicode-bidi`],[`unicodeRange`,`unicode-range`],[`unitsPerEm`,`units-per-em`],[`vAlphabetic`,`v-alphabetic`],[`vHanging`,`v-hanging`],[`vIdeographic`,`v-ideographic`],[`vMathematical`,`v-mathematical`],[`vectorEffect`,`vector-effect`],[`vertAdvY`,`vert-adv-y`],[`vertOriginX`,`vert-origin-x`],[`vertOriginY`,`vert-origin-y`],[`wordSpacing`,`word-spacing`],[`writingMode`,`writing-mode`],[`xmlnsXlink`,`xmlns:xlink`],[`xHeight`,`x-height`]]),sn=/^[\u0000-\u001F ]*j[\r\n\t]*a[\r\n\t]*v[\r\n\t]*a[\r\n\t]*s[\r\n\t]*c[\r\n\t]*r[\r\n\t]*i[\r\n\t]*p[\r\n\t]*t[\r\n\t]*:/i;function cn(e){return sn.test(``+e)?`javascript:throw new Error('React has blocked a javascript: URL as a security precaution.')`:e}function ln(){}var un=null;function dn(e){return e=e.target||e.srcElement||window,e.correspondingUseElement&&(e=e.correspondingUseElement),e.nodeType===3?e.parentNode:e}var fn=null,pn=null;function mn(e){var t=Et(e);if(t&&(e=t.stateNode)){var n=e[_t]||null;a:switch(e=t.stateNode,t.type){case`input`:if(Yt(e,n.value,n.defaultValue,n.defaultValue,n.checked,n.defaultChecked,n.type,n.name),t=n.name,n.type===`radio`&&t!=null){for(n=e;n.parentNode;)n=n.parentNode;for(n=n.querySelectorAll(`input[name="`+Jt(``+t)+`"][type="radio"]`),t=0;t<n.length;t++){var r=n[t];if(r!==e&&r.form===e.form){var i=r[_t]||null;if(!i)throw Error(s(90));Yt(r,i.value,i.defaultValue,i.defaultValue,i.checked,i.defaultChecked,i.type,i.name)}}for(t=0;t<n.length;t++)r=n[t],r.form===e.form&&Gt(r)}break a;case`textarea`:$t(e,n.value,n.defaultValue);break a;case`select`:t=n.value,t!=null&&Qt(e,!!n.multiple,t,!1)}}}var hn=!1;function gn(e,t,n){if(hn)return e(t,n);hn=!0;try{return e(t)}finally{if(hn=!1,(fn!==null||pn!==null)&&(wu(),fn&&(t=fn,e=pn,pn=fn=null,mn(t),e)))for(t=0;t<e.length;t++)mn(e[t])}}function _n(e,t){var n=e.stateNode;if(n===null)return null;var r=n[_t]||null;if(r===null)return null;n=r[t];a:switch(t){case`onClick`:case`onClickCapture`:case`onDoubleClick`:case`onDoubleClickCapture`:case`onMouseDown`:case`onMouseDownCapture`:case`onMouseMove`:case`onMouseMoveCapture`:case`onMouseUp`:case`onMouseUpCapture`:case`onMouseEnter`:(r=!r.disabled)||(e=e.type,r=!(e===`button`||e===`input`||e===`select`||e===`textarea`)),e=!r;break a;default:e=!1}if(e)return null;if(n&&typeof n!=`function`)throw Error(s(231,t,typeof n));return n}var vn=!(typeof window>`u`||window.document===void 0||window.document.createElement===void 0),yn=!1;if(vn)try{var bn={};Object.defineProperty(bn,"passive",{get:function(){yn=!0}}),window.addEventListener(`test`,bn,bn),window.removeEventListener(`test`,bn,bn)}catch{yn=!1}var xn=null,Sn=null,Cn=null;function wn(){if(Cn)return Cn;var e,t=Sn,n=t.length,r,i=`value`in xn?xn.value:xn.textContent,a=i.length;for(e=0;e<n&&t[e]===i[e];e++);var o=n-e;for(r=1;r<=o&&t[n-r]===i[a-r];r++);return Cn=i.slice(e,1<r?1-r:void 0)}function Tn(e){var t=e.keyCode;return`charCode`in e?(e=e.charCode,e===0&&t===13&&(e=13)):e=t,e===10&&(e=13),32<=e||e===13?e:0}function En(){return!0}function Dn(){return!1}function On(e){function t(t,n,r,i,a){for(var o in this._reactName=t,this._targetInst=r,this.type=n,this.nativeEvent=i,this.target=a,this.currentTarget=null,e)e.hasOwnProperty(o)&&(t=e[o],this[o]=t?t(i):i[o]);return this.isDefaultPrevented=(i.defaultPrevented==null?!1===i.returnValue:i.defaultPrevented)?En:Dn,this.isPropagationStopped=Dn,this}return h(t.prototype,{preventDefault:function(){this.defaultPrevented=!0;var e=this.nativeEvent;e&&(e.preventDefault?e.preventDefault():typeof e.returnValue!=`unknown`&&(e.returnValue=!1),this.isDefaultPrevented=En)},stopPropagation:function(){var e=this.nativeEvent;e&&(e.stopPropagation?e.stopPropagation():typeof e.cancelBubble!=`unknown`&&(e.cancelBubble=!0),this.isPropagationStopped=En)},persist:function(){},isPersistent:En}),t}var kn={eventPhase:0,bubbles:0,cancelable:0,timeStamp:function(e){return e.timeStamp||Date.now()},defaultPrevented:0,isTrusted:0},An=On(kn),jn=h({},kn,{view:0,detail:0}),Mn=On(jn),Nn,Pn,Fn,In=h({},jn,{screenX:0,screenY:0,clientX:0,clientY:0,pageX:0,pageY:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,getModifierState:qn,button:0,buttons:0,relatedTarget:function(e){return e.relatedTarget===void 0?e.fromElement===e.srcElement?e.toElement:e.fromElement:e.relatedTarget},movementX:function(e){return`movementX`in e?e.movementX:(e!==Fn&&(Fn&&e.type===`mousemove`?(Nn=e.screenX-Fn.screenX,Pn=e.screenY-Fn.screenY):Pn=Nn=0,Fn=e),Nn)},movementY:function(e){return`movementY`in e?e.movementY:Pn}}),Ln=On(In),Rn=On(h({},In,{dataTransfer:0})),zn=On(h({},jn,{relatedTarget:0})),Bn=On(h({},kn,{animationName:0,elapsedTime:0,pseudoElement:0})),Vn=On(h({},kn,{clipboardData:function(e){return`clipboardData`in e?e.clipboardData:window.clipboardData}})),Hn=On(h({},kn,{data:0})),Un={Esc:`Escape`,Spacebar:` `,Left:`ArrowLeft`,Up:`ArrowUp`,Right:`ArrowRight`,Down:`ArrowDown`,Del:`Delete`,Win:`OS`,Menu:`ContextMenu`,Apps:`ContextMenu`,Scroll:`ScrollLock`,MozPrintableKey:`Unidentified`},Wn={8:`Backspace`,9:`Tab`,12:`Clear`,13:`Enter`,16:`Shift`,17:`Control`,18:`Alt`,19:`Pause`,20:`CapsLock`,27:`Escape`,32:` `,33:`PageUp`,34:`PageDown`,35:`End`,36:`Home`,37:`ArrowLeft`,38:`ArrowUp`,39:`ArrowRight`,40:`ArrowDown`,45:`Insert`,46:`Delete`,112:`F1`,113:`F2`,114:`F3`,115:`F4`,116:`F5`,117:`F6`,118:`F7`,119:`F8`,120:`F9`,121:`F10`,122:`F11`,123:`F12`,144:`NumLock`,145:`ScrollLock`,224:`Meta`},Gn={Alt:`altKey`,Control:`ctrlKey`,Meta:`metaKey`,Shift:`shiftKey`};function Kn(e){var t=this.nativeEvent;return t.getModifierState?t.getModifierState(e):(e=Gn[e])?!!t[e]:!1}function qn(){return Kn}var Jn=On(h({},jn,{key:function(e){if(e.key){var t=Un[e.key]||e.key;if(t!==`Unidentified`)return t}return e.type===`keypress`?(e=Tn(e),e===13?`Enter`:String.fromCharCode(e)):e.type===`keydown`||e.type===`keyup`?Wn[e.keyCode]||`Unidentified`:``},code:0,location:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,repeat:0,locale:0,getModifierState:qn,charCode:function(e){return e.type===`keypress`?Tn(e):0},keyCode:function(e){return e.type===`keydown`||e.type===`keyup`?e.keyCode:0},which:function(e){return e.type===`keypress`?Tn(e):e.type===`keydown`||e.type===`keyup`?e.keyCode:0}})),Yn=On(h({},In,{pointerId:0,width:0,height:0,pressure:0,tangentialPressure:0,tiltX:0,tiltY:0,twist:0,pointerType:0,isPrimary:0})),Xn=On(h({},jn,{touches:0,targetTouches:0,changedTouches:0,altKey:0,metaKey:0,ctrlKey:0,shiftKey:0,getModifierState:qn})),Zn=On(h({},kn,{propertyName:0,elapsedTime:0,pseudoElement:0})),Qn=On(h({},In,{deltaX:function(e){return`deltaX`in e?e.deltaX:`wheelDeltaX`in e?-e.wheelDeltaX:0},deltaY:function(e){return`deltaY`in e?e.deltaY:`wheelDeltaY`in e?-e.wheelDeltaY:`wheelDelta`in e?-e.wheelDelta:0},deltaZ:0,deltaMode:0})),$n=On(h({},kn,{newState:0,oldState:0})),er=[9,13,27,32],tr=vn&&`CompositionEvent`in window,nr=null;vn&&`documentMode`in document&&(nr=document.documentMode);var rr=vn&&`TextEvent`in window&&!nr,ir=vn&&(!tr||nr&&8<nr&&11>=nr),ar=` `,or=!1;function sr(e,t){switch(e){case`keyup`:return er.indexOf(t.keyCode)!==-1;case`keydown`:return t.keyCode!==229;case`keypress`:case`mousedown`:case`focusout`:return!0;default:return!1}}function cr(e){return e=e.detail,typeof e==`object`&&`data`in e?e.data:null}var lr=!1;function ur(e,t){switch(e){case`compositionend`:return cr(t);case`keypress`:return t.which===32?(or=!0,ar):null;case`textInput`:return e=t.data,e===ar&&or?null:e;default:return null}}function dr(e,t){if(lr)return e===`compositionend`||!tr&&sr(e,t)?(e=wn(),Cn=Sn=xn=null,lr=!1,e):null;switch(e){case`paste`:return null;case`keypress`:if(!(t.ctrlKey||t.altKey||t.metaKey)||t.ctrlKey&&t.altKey){if(t.char&&1<t.char.length)return t.char;if(t.which)return String.fromCharCode(t.which)}return null;case`compositionend`:return ir&&t.locale!==`ko`?null:t.data;default:return null}}var fr={color:!0,date:!0,datetime:!0,"datetime-local":!0,email:!0,month:!0,number:!0,password:!0,range:!0,search:!0,tel:!0,text:!0,time:!0,url:!0,week:!0};function pr(e){var t=e&&e.nodeName&&e.nodeName.toLowerCase();return t===`input`?!!fr[e.type]:t===`textarea`}function mr(e,t,n,r){fn?pn?pn.push(r):pn=[r]:fn=r,t=kd(t,`onChange`),0<t.length&&(n=new An(`onChange`,`change`,null,n,r),e.push({event:n,listeners:t}))}var hr=null,gr=null;function _r(e){Sd(e,0)}function vr(e){if(Gt(Dt(e)))return e}function yr(e,t){if(e===`change`)return t}var br=!1;if(vn){var xr;if(vn){var Sr=`oninput`in document;if(!Sr){var A=document.createElement(`div`);A.setAttribute(`oninput`,`return;`),Sr=typeof A.oninput==`function`}xr=Sr}else xr=!1;br=xr&&(!document.documentMode||9<document.documentMode)}function Cr(){hr&&(hr.detachEvent(`onpropertychange`,wr),gr=hr=null)}function wr(e){if(e.propertyName===`value`&&vr(gr)){var t=[];mr(t,gr,e,dn(e)),gn(_r,t)}}function Tr(e,t,n){e===`focusin`?(Cr(),hr=t,gr=n,hr.attachEvent(`onpropertychange`,wr)):e===`focusout`&&Cr()}function Er(e){if(e===`selectionchange`||e===`keyup`||e===`keydown`)return vr(gr)}function Dr(e,t){if(e===`click`)return vr(t)}function Or(e,t){if(e===`input`||e===`change`)return vr(t)}function kr(e,t){return e===t&&(e!==0||1/e==1/t)||e!==e&&t!==t}var Ar=typeof Object.is==`function`?Object.is:kr;function jr(e,t){if(Ar(e,t))return!0;if(typeof e!=`object`||!e||typeof t!=`object`||!t)return!1;var n=Object.keys(e),r=Object.keys(t);if(n.length!==r.length)return!1;for(r=0;r<n.length;r++){var i=n[r];if(!Ae.call(t,i)||!Ar(e[i],t[i]))return!1}return!0}function Mr(e){for(;e&&e.firstChild;)e=e.firstChild;return e}function Nr(e,t){var n=Mr(e);e=0;for(var r;n;){if(n.nodeType===3){if(r=e+n.textContent.length,e<=t&&r>=t)return{node:n,offset:t-e};e=r}a:{for(;n;){if(n.nextSibling){n=n.nextSibling;break a}n=n.parentNode}n=void 0}n=Mr(n)}}function Pr(e,t){return e&&t?e===t?!0:e&&e.nodeType===3?!1:t&&t.nodeType===3?Pr(e,t.parentNode):`contains`in e?e.contains(t):e.compareDocumentPosition?!!(e.compareDocumentPosition(t)&16):!1:!1}function Fr(e){e=e!=null&&e.ownerDocument!=null&&e.ownerDocument.defaultView!=null?e.ownerDocument.defaultView:window;for(var t=Kt(e.document);t instanceof e.HTMLIFrameElement;){try{var n=typeof t.contentWindow.location.href==`string`}catch{n=!1}if(n)e=t.contentWindow;else break;t=Kt(e.document)}return t}function Ir(e){var t=e&&e.nodeName&&e.nodeName.toLowerCase();return t&&(t===`input`&&(e.type===`text`||e.type===`search`||e.type===`tel`||e.type===`url`||e.type===`password`)||t===`textarea`||e.contentEditable===`true`)}var Lr=vn&&`documentMode`in document&&11>=document.documentMode,Rr=null,zr=null,Br=null,Vr=!1;function Hr(e,t,n){var r=n.window===n?n.document:n.nodeType===9?n:n.ownerDocument;Vr||Rr==null||Rr!==Kt(r)||(r=Rr,`selectionStart`in r&&Ir(r)?r={start:r.selectionStart,end:r.selectionEnd}:(r=(r.ownerDocument&&r.ownerDocument.defaultView||window).getSelection(),r={anchorNode:r.anchorNode,anchorOffset:r.anchorOffset,focusNode:r.focusNode,focusOffset:r.focusOffset}),Br&&jr(Br,r)||(Br=r,r=kd(zr,`onSelect`),0<r.length&&(t=new An(`onSelect`,`select`,null,t,n),e.push({event:t,listeners:r}),t.target=Rr)))}function Ur(e,t){var n={};return n[e.toLowerCase()]=t.toLowerCase(),n[`Webkit`+e]=`webkit`+t,n[`Moz`+e]=`moz`+t,n}var Wr={animationend:Ur(`Animation`,`AnimationEnd`),animationiteration:Ur(`Animation`,`AnimationIteration`),animationstart:Ur(`Animation`,`AnimationStart`),transitionrun:Ur(`Transition`,`TransitionRun`),transitionstart:Ur(`Transition`,`TransitionStart`),transitioncancel:Ur(`Transition`,`TransitionCancel`),transitionend:Ur(`Transition`,`TransitionEnd`)},j={},Gr={};vn&&(Gr=document.createElement(`div`).style,`AnimationEvent`in window||(delete Wr.animationend.animation,delete Wr.animationiteration.animation,delete Wr.animationstart.animation),`TransitionEvent`in window||delete Wr.transitionend.transition);function Kr(e){if(j[e])return j[e];if(!Wr[e])return e;var t=Wr[e],n;for(n in t)if(t.hasOwnProperty(n)&&n in Gr)return j[e]=t[n];return e}var qr=Kr(`animationend`),Jr=Kr(`animationiteration`),Yr=Kr(`animationstart`),Xr=Kr(`transitionrun`),Zr=Kr(`transitionstart`),Qr=Kr(`transitioncancel`),$r=Kr(`transitionend`),ei=new Map,ti=`abort auxClick beforeToggle cancel canPlay canPlayThrough click close contextMenu copy cut drag dragEnd dragEnter dragExit dragLeave dragOver dragStart drop durationChange emptied encrypted ended error gotPointerCapture input invalid keyDown keyPress keyUp load loadedData loadedMetadata loadStart lostPointerCapture mouseDown mouseMove mouseOut mouseOver mouseUp paste pause play playing pointerCancel pointerDown pointerMove pointerOut pointerOver pointerUp progress rateChange reset resize seeked seeking stalled submit suspend timeUpdate touchCancel touchEnd touchStart volumeChange scroll toggle touchMove waiting wheel`.split(` `);ti.push(`scrollEnd`);function ni(e,t){ei.set(e,t),Mt(t,[e])}var ri=typeof reportError==`function`?reportError:function(e){if(typeof window==`object`&&typeof window.ErrorEvent==`function`){var t=new window.ErrorEvent(`error`,{bubbles:!0,cancelable:!0,message:typeof e==`object`&&e&&typeof e.message==`string`?String(e.message):String(e),error:e});if(!window.dispatchEvent(t))return}else if(typeof process==`object`&&typeof process.emit==`function`){process.emit(`uncaughtException`,e);return}console.error(e)},ii=[],ai=0,oi=0;function si(){for(var e=ai,t=oi=ai=0;t<e;){var n=ii[t];ii[t++]=null;var r=ii[t];ii[t++]=null;var i=ii[t];ii[t++]=null;var a=ii[t];if(ii[t++]=null,r!==null&&i!==null){var o=r.pending;o===null?i.next=i:(i.next=o.next,o.next=i),r.pending=i}a!==0&&di(n,i,a)}}function ci(e,t,n,r){ii[ai++]=e,ii[ai++]=t,ii[ai++]=n,ii[ai++]=r,oi|=r,e.lanes|=r,e=e.alternate,e!==null&&(e.lanes|=r)}function li(e,t,n,r){return ci(e,t,n,r),fi(e)}function ui(e,t){return ci(e,null,null,t),fi(e)}function di(e,t,n){e.lanes|=n;var r=e.alternate;r!==null&&(r.lanes|=n);for(var i=!1,a=e.return;a!==null;)a.childLanes|=n,r=a.alternate,r!==null&&(r.childLanes|=n),a.tag===22&&(e=a.stateNode,e===null||e._visibility&1||(i=!0)),e=a,a=a.return;return e.tag===3?(a=e.stateNode,i&&t!==null&&(i=31-qe(n),e=a.hiddenUpdates,r=e[i],r===null?e[i]=[t]:r.push(t),t.lane=n|536870912),a):null}function fi(e){if(50<gu)throw gu=0,_u=null,Error(s(185));for(var t=e.return;t!==null;)e=t,t=e.return;return e.tag===3?e.stateNode:null}var pi={};function mi(e,t,n,r){this.tag=e,this.key=n,this.sibling=this.child=this.return=this.stateNode=this.type=this.elementType=null,this.index=0,this.refCleanup=this.ref=null,this.pendingProps=t,this.dependencies=this.memoizedState=this.updateQueue=this.memoizedProps=null,this.mode=r,this.subtreeFlags=this.flags=0,this.deletions=null,this.childLanes=this.lanes=0,this.alternate=null}function hi(e,t,n,r){return new mi(e,t,n,r)}function gi(e){return e=e.prototype,!(!e||!e.isReactComponent)}function _i(e,t){var n=e.alternate;return n===null?(n=hi(e.tag,t,e.key,e.mode),n.elementType=e.elementType,n.type=e.type,n.stateNode=e.stateNode,n.alternate=e,e.alternate=n):(n.pendingProps=t,n.type=e.type,n.flags=0,n.subtreeFlags=0,n.deletions=null),n.flags=e.flags&65011712,n.childLanes=e.childLanes,n.lanes=e.lanes,n.child=e.child,n.memoizedProps=e.memoizedProps,n.memoizedState=e.memoizedState,n.updateQueue=e.updateQueue,t=e.dependencies,n.dependencies=t===null?null:{lanes:t.lanes,firstContext:t.firstContext},n.sibling=e.sibling,n.index=e.index,n.ref=e.ref,n.refCleanup=e.refCleanup,n}function vi(e,t){e.flags&=65011714;var n=e.alternate;return n===null?(e.childLanes=0,e.lanes=t,e.child=null,e.subtreeFlags=0,e.memoizedProps=null,e.memoizedState=null,e.updateQueue=null,e.dependencies=null,e.stateNode=null):(e.childLanes=n.childLanes,e.lanes=n.lanes,e.child=n.child,e.subtreeFlags=0,e.deletions=null,e.memoizedProps=n.memoizedProps,e.memoizedState=n.memoizedState,e.updateQueue=n.updateQueue,e.type=n.type,t=n.dependencies,e.dependencies=t===null?null:{lanes:t.lanes,firstContext:t.firstContext}),e}function yi(e,t,n,r,i,a){var o=0;if(r=e,typeof e==`function`)gi(e)&&(o=1);else if(typeof e==`string`)o=Gf(e,n,he.current)?26:e===`html`||e===`head`||e===`body`?27:5;else a:switch(e){case re:return e=hi(31,n,t,i),e.elementType=re,e.lanes=a,e;case y:return bi(n.children,i,a,t);case b:o=8,i|=24;break;case x:return e=hi(12,n,t,i|2),e.elementType=x,e.lanes=a,e;case ee:return e=hi(13,n,t,i),e.elementType=ee,e.lanes=a,e;case te:return e=hi(19,n,t,i),e.elementType=te,e.lanes=a,e;default:if(typeof e==`object`&&e)switch(e.$$typeof){case C:o=10;break a;case S:o=9;break a;case w:o=11;break a;case T:o=14;break a;case ne:o=16,r=null;break a}o=29,n=Error(s(130,e===null?`null`:typeof e,``)),r=null}return t=hi(o,n,t,i),t.elementType=e,t.type=r,t.lanes=a,t}function bi(e,t,n,r){return e=hi(7,e,r,t),e.lanes=n,e}function xi(e,t,n){return e=hi(6,e,null,t),e.lanes=n,e}function Si(e){var t=hi(18,null,null,0);return t.stateNode=e,t}function Ci(e,t,n){return t=hi(4,e.children===null?[]:e.children,e.key,t),t.lanes=n,t.stateNode={containerInfo:e.containerInfo,pendingChildren:null,implementation:e.implementation},t}var wi=new WeakMap;function Ti(e,t){if(typeof e==`object`&&e){var n=wi.get(e);return n===void 0?(t={value:e,source:t,stack:ke(t)},wi.set(e,t),t):n}return{value:e,source:t,stack:ke(t)}}var Ei=[],Di=0,Oi=null,ki=0,Ai=[],ji=0,Mi=null,Ni=1,Pi=``;function Fi(e,t){Ei[Di++]=ki,Ei[Di++]=Oi,Oi=e,ki=t}function Ii(e,t,n){Ai[ji++]=Ni,Ai[ji++]=Pi,Ai[ji++]=Mi,Mi=e;var r=Ni;e=Pi;var i=32-qe(r)-1;r&=~(1<<i),n+=1;var a=32-qe(t)+i;if(30<a){var o=i-i%5;a=(r&(1<<o)-1).toString(32),r>>=o,i-=o,Ni=1<<32-qe(t)+i|n<<i|r,Pi=a+e}else Ni=1<<a|n<<i|r,Pi=e}function Li(e){e.return!==null&&(Fi(e,1),Ii(e,1,0))}function Ri(e){for(;e===Oi;)Oi=Ei[--Di],Ei[Di]=null,ki=Ei[--Di],Ei[Di]=null;for(;e===Mi;)Mi=Ai[--ji],Ai[ji]=null,Pi=Ai[--ji],Ai[ji]=null,Ni=Ai[--ji],Ai[ji]=null}function zi(e,t){Ai[ji++]=Ni,Ai[ji++]=Pi,Ai[ji++]=Mi,Ni=t.id,Pi=t.overflow,Mi=e}var Bi=null,Vi=null,M=!1,Hi=null,Ui=!1,Wi=Error(s(519));function Gi(e){throw Zi(Ti(Error(s(418,1<arguments.length&&arguments[1]!==void 0&&arguments[1]?`text`:`HTML`,``)),e)),Wi}function Ki(e){var t=e.stateNode,n=e.type,r=e.memoizedProps;switch(t[gt]=e,t[_t]=r,n){case`dialog`:Y(`cancel`,t),Y(`close`,t);break;case`iframe`:case`object`:case`embed`:Y(`load`,t);break;case`video`:case`audio`:for(n=0;n<bd.length;n++)Y(bd[n],t);break;case`source`:Y(`error`,t);break;case`img`:case`image`:case`link`:Y(`error`,t),Y(`load`,t);break;case`details`:Y(`toggle`,t);break;case`input`:Y(`invalid`,t),Xt(t,r.value,r.defaultValue,r.checked,r.defaultChecked,r.type,r.name,!0);break;case`select`:Y(`invalid`,t);break;case`textarea`:Y(`invalid`,t),en(t,r.value,r.defaultValue,r.children)}n=r.children,typeof n!=`string`&&typeof n!=`number`&&typeof n!=`bigint`||t.textContent===``+n||!0===r.suppressHydrationWarning||Fd(t.textContent,n)?(r.popover!=null&&(Y(`beforetoggle`,t),Y(`toggle`,t)),r.onScroll!=null&&Y(`scroll`,t),r.onScrollEnd!=null&&Y(`scrollend`,t),r.onClick!=null&&(t.onclick=ln),t=!0):t=!1,t||Gi(e,!0)}function qi(e){for(Bi=e.return;Bi;)switch(Bi.tag){case 5:case 31:case 13:Ui=!1;return;case 27:case 3:Ui=!0;return;default:Bi=Bi.return}}function Ji(e){if(e!==Bi)return!1;if(!M)return qi(e),M=!0,!1;var t=e.tag,n;if((n=t!==3&&t!==27)&&((n=t===5)&&(n=e.type,n=!(n!==`form`&&n!==`button`)||Kd(e.type,e.memoizedProps)),n=!n),n&&Vi&&Gi(e),qi(e),t===13){if(e=e.memoizedState,e=e===null?null:e.dehydrated,!e)throw Error(s(317));Vi=pf(e)}else if(t===31){if(e=e.memoizedState,e=e===null?null:e.dehydrated,!e)throw Error(s(317));Vi=pf(e)}else t===27?(t=Vi,ef(e.type)?(e=ff,ff=null,Vi=e):Vi=t):Vi=Bi?df(e.stateNode.nextSibling):null;return!0}function Yi(){Vi=Bi=null,M=!1}function Xi(){var e=Hi;return e!==null&&(ru===null?ru=e:ru.push.apply(ru,e),Hi=null),e}function Zi(e){Hi===null?Hi=[e]:Hi.push(e)}var Qi=fe(null),$i=null,ea=null;function ta(e,t,n){me(Qi,t._currentValue),t._currentValue=n}function na(e){e._currentValue=Qi.current,pe(Qi)}function ra(e,t,n){for(;e!==null;){var r=e.alternate;if((e.childLanes&t)===t?r!==null&&(r.childLanes&t)!==t&&(r.childLanes|=t):(e.childLanes|=t,r!==null&&(r.childLanes|=t)),e===n)break;e=e.return}}function N(e,t,n,r){var i=e.child;for(i!==null&&(i.return=e);i!==null;){var a=i.dependencies;if(a!==null){var o=i.child;a=a.firstContext;a:for(;a!==null;){var c=a;a=i;for(var l=0;l<t.length;l++)if(c.context===t[l]){a.lanes|=n,c=a.alternate,c!==null&&(c.lanes|=n),ra(a.return,n,e),r||(o=null);break a}a=c.next}}else if(i.tag===18){if(o=i.return,o===null)throw Error(s(341));o.lanes|=n,a=o.alternate,a!==null&&(a.lanes|=n),ra(o,n,e),o=null}else o=i.child;if(o!==null)o.return=i;else for(o=i;o!==null;){if(o===e){o=null;break}if(i=o.sibling,i!==null){i.return=o.return,o=i;break}o=o.return}i=o}}function ia(e,t,n,r){e=null;for(var i=t,a=!1;i!==null;){if(!a){if(i.flags&524288)a=!0;else if(i.flags&262144)break}if(i.tag===10){var o=i.alternate;if(o===null)throw Error(s(387));if(o=o.memoizedProps,o!==null){var c=i.type;Ar(i.pendingProps.value,o.value)||(e===null?e=[c]:e.push(c))}}else if(i===ve.current){if(o=i.alternate,o===null)throw Error(s(387));o.memoizedState.memoizedState!==i.memoizedState.memoizedState&&(e===null?e=[ep]:e.push(ep))}i=i.return}e!==null&&N(t,e,n,r),t.flags|=262144}function aa(e){for(e=e.firstContext;e!==null;){if(!Ar(e.context._currentValue,e.memoizedValue))return!0;e=e.next}return!1}function oa(e){$i=e,ea=null,e=e.dependencies,e!==null&&(e.firstContext=null)}function sa(e){return la($i,e)}function ca(e,t){return $i===null&&oa(e),la(e,t)}function la(e,t){var n=t._currentValue;if(t={context:t,memoizedValue:n,next:null},ea===null){if(e===null)throw Error(s(308));ea=t,e.dependencies={lanes:0,firstContext:t},e.flags|=524288}else ea=ea.next=t;return n}var ua=typeof AbortController<`u`?AbortController:function(){var e=[],t=this.signal={aborted:!1,addEventListener:function(t,n){e.push(n)}};this.abort=function(){t.aborted=!0,e.forEach(function(e){return e()})}},da=t.unstable_scheduleCallback,fa=t.unstable_NormalPriority,pa={$$typeof:C,Consumer:null,Provider:null,_currentValue:null,_currentValue2:null,_threadCount:0};function ma(){return{controller:new ua,data:new Map,refCount:0}}function ha(e){e.refCount--,e.refCount===0&&da(fa,function(){e.controller.abort()})}var ga=null,_a=0,va=0,ya=null;function ba(e,t){if(ga===null){var n=ga=[];_a=0,va=md(),ya={status:`pending`,value:void 0,then:function(e){n.push(e)}}}return _a++,t.then(xa,xa),t}function xa(){if(--_a===0&&ga!==null){ya!==null&&(ya.status=`fulfilled`);var e=ga;ga=null,va=0,ya=null;for(var t=0;t<e.length;t++)(0,e[t])()}}function Sa(e,t){var n=[],r={status:`pending`,value:null,reason:null,then:function(e){n.push(e)}};return e.then(function(){r.status=`fulfilled`,r.value=t;for(var e=0;e<n.length;e++)(0,n[e])(t)},function(e){for(r.status=`rejected`,r.reason=e,e=0;e<n.length;e++)(0,n[e])(void 0)}),r}var Ca=D.S;D.S=function(e,t){ou=Fe(),typeof t==`object`&&t&&typeof t.then==`function`&&ba(e,t),Ca!==null&&Ca(e,t)};var wa=fe(null);function Ta(){var e=wa.current;return e===null?Gl.pooledCache:e}function Ea(e,t){t===null?me(wa,wa.current):me(wa,t.pool)}function Da(){var e=Ta();return e===null?null:{parent:pa._currentValue,pool:e}}var Oa=Error(s(460)),ka=Error(s(474)),Aa=Error(s(542)),ja={then:function(){}};function Ma(e){return e=e.status,e===`fulfilled`||e===`rejected`}function Na(e,t,n){switch(n=e[n],n===void 0?e.push(t):n!==t&&(t.then(ln,ln),t=n),t.status){case`fulfilled`:return t.value;case`rejected`:throw e=t.reason,La(e),e;default:if(typeof t.status==`string`)t.then(ln,ln);else{if(e=Gl,e!==null&&100<e.shellSuspendCounter)throw Error(s(482));e=t,e.status=`pending`,e.then(function(e){if(t.status===`pending`){var n=t;n.status=`fulfilled`,n.value=e}},function(e){if(t.status===`pending`){var n=t;n.status=`rejected`,n.reason=e}})}switch(t.status){case`fulfilled`:return t.value;case`rejected`:throw e=t.reason,La(e),e}throw Fa=t,Oa}}function Pa(e){try{var t=e._init;return t(e._payload)}catch(e){throw typeof e==`object`&&e&&typeof e.then==`function`?(Fa=e,Oa):e}}var Fa=null;function Ia(){if(Fa===null)throw Error(s(459));var e=Fa;return Fa=null,e}function La(e){if(e===Oa||e===Aa)throw Error(s(483))}var Ra=null,za=0;function Ba(e){var t=za;return za+=1,Ra===null&&(Ra=[]),Na(Ra,e,t)}function Va(e,t){t=t.props.ref,e.ref=t===void 0?null:t}function Ha(e,t){throw t.$$typeof===g?Error(s(525)):(e=Object.prototype.toString.call(t),Error(s(31,e===`[object Object]`?`object with keys {`+Object.keys(t).join(`, `)+`}`:e)))}function Ua(e){function t(t,n){if(e){var r=t.deletions;r===null?(t.deletions=[n],t.flags|=16):r.push(n)}}function n(n,r){if(!e)return null;for(;r!==null;)t(n,r),r=r.sibling;return null}function r(e){for(var t=new Map;e!==null;)e.key===null?t.set(e.index,e):t.set(e.key,e),e=e.sibling;return t}function i(e,t){return e=_i(e,t),e.index=0,e.sibling=null,e}function a(t,n,r){return t.index=r,e?(r=t.alternate,r===null?(t.flags|=67108866,n):(r=r.index,r<n?(t.flags|=67108866,n):r)):(t.flags|=1048576,n)}function o(t){return e&&t.alternate===null&&(t.flags|=67108866),t}function c(e,t,n,r){return t===null||t.tag!==6?(t=xi(n,e.mode,r),t.return=e,t):(t=i(t,n),t.return=e,t)}function l(e,t,n,r){var a=n.type;return a===y?d(e,t,n.props.children,r,n.key):t!==null&&(t.elementType===a||typeof a==`object`&&a&&a.$$typeof===ne&&Pa(a)===t.type)?(t=i(t,n.props),Va(t,n),t.return=e,t):(t=yi(n.type,n.key,n.props,null,e.mode,r),Va(t,n),t.return=e,t)}function u(e,t,n,r){return t===null||t.tag!==4||t.stateNode.containerInfo!==n.containerInfo||t.stateNode.implementation!==n.implementation?(t=Ci(n,e.mode,r),t.return=e,t):(t=i(t,n.children||[]),t.return=e,t)}function d(e,t,n,r,a){return t===null||t.tag!==7?(t=bi(n,e.mode,r,a),t.return=e,t):(t=i(t,n),t.return=e,t)}function f(e,t,n){if(typeof t==`string`&&t!==``||typeof t==`number`||typeof t==`bigint`)return t=xi(``+t,e.mode,n),t.return=e,t;if(typeof t==`object`&&t){switch(t.$$typeof){case _:return n=yi(t.type,t.key,t.props,null,e.mode,n),Va(n,t),n.return=e,n;case v:return t=Ci(t,e.mode,n),t.return=e,t;case ne:return t=Pa(t),f(e,t,n)}if(ce(t)||ae(t))return t=bi(t,e.mode,n,null),t.return=e,t;if(typeof t.then==`function`)return f(e,Ba(t),n);if(t.$$typeof===C)return f(e,ca(e,t),n);Ha(e,t)}return null}function p(e,t,n,r){var i=t===null?null:t.key;if(typeof n==`string`&&n!==``||typeof n==`number`||typeof n==`bigint`)return i===null?c(e,t,``+n,r):null;if(typeof n==`object`&&n){switch(n.$$typeof){case _:return n.key===i?l(e,t,n,r):null;case v:return n.key===i?u(e,t,n,r):null;case ne:return n=Pa(n),p(e,t,n,r)}if(ce(n)||ae(n))return i===null?d(e,t,n,r,null):null;if(typeof n.then==`function`)return p(e,t,Ba(n),r);if(n.$$typeof===C)return p(e,t,ca(e,n),r);Ha(e,n)}return null}function m(e,t,n,r,i){if(typeof r==`string`&&r!==``||typeof r==`number`||typeof r==`bigint`)return e=e.get(n)||null,c(t,e,``+r,i);if(typeof r==`object`&&r){switch(r.$$typeof){case _:return e=e.get(r.key===null?n:r.key)||null,l(t,e,r,i);case v:return e=e.get(r.key===null?n:r.key)||null,u(t,e,r,i);case ne:return r=Pa(r),m(e,t,n,r,i)}if(ce(r)||ae(r))return e=e.get(n)||null,d(t,e,r,i,null);if(typeof r.then==`function`)return m(e,t,n,Ba(r),i);if(r.$$typeof===C)return m(e,t,n,ca(t,r),i);Ha(t,r)}return null}function h(i,o,s,c){for(var l=null,u=null,d=o,h=o=0,g=null;d!==null&&h<s.length;h++){d.index>h?(g=d,d=null):g=d.sibling;var _=p(i,d,s[h],c);if(_===null){d===null&&(d=g);break}e&&d&&_.alternate===null&&t(i,d),o=a(_,o,h),u===null?l=_:u.sibling=_,u=_,d=g}if(h===s.length)return n(i,d),M&&Fi(i,h),l;if(d===null){for(;h<s.length;h++)d=f(i,s[h],c),d!==null&&(o=a(d,o,h),u===null?l=d:u.sibling=d,u=d);return M&&Fi(i,h),l}for(d=r(d);h<s.length;h++)g=m(d,i,h,s[h],c),g!==null&&(e&&g.alternate!==null&&d.delete(g.key===null?h:g.key),o=a(g,o,h),u===null?l=g:u.sibling=g,u=g);return e&&d.forEach(function(e){return t(i,e)}),M&&Fi(i,h),l}function g(i,o,c,l){if(c==null)throw Error(s(151));for(var u=null,d=null,h=o,g=o=0,_=null,v=c.next();h!==null&&!v.done;g++,v=c.next()){h.index>g?(_=h,h=null):_=h.sibling;var y=p(i,h,v.value,l);if(y===null){h===null&&(h=_);break}e&&h&&y.alternate===null&&t(i,h),o=a(y,o,g),d===null?u=y:d.sibling=y,d=y,h=_}if(v.done)return n(i,h),M&&Fi(i,g),u;if(h===null){for(;!v.done;g++,v=c.next())v=f(i,v.value,l),v!==null&&(o=a(v,o,g),d===null?u=v:d.sibling=v,d=v);return M&&Fi(i,g),u}for(h=r(h);!v.done;g++,v=c.next())v=m(h,i,g,v.value,l),v!==null&&(e&&v.alternate!==null&&h.delete(v.key===null?g:v.key),o=a(v,o,g),d===null?u=v:d.sibling=v,d=v);return e&&h.forEach(function(e){return t(i,e)}),M&&Fi(i,g),u}function b(e,r,a,c){if(typeof a==`object`&&a&&a.type===y&&a.key===null&&(a=a.props.children),typeof a==`object`&&a){switch(a.$$typeof){case _:a:{for(var l=a.key;r!==null;){if(r.key===l){if(l=a.type,l===y){if(r.tag===7){n(e,r.sibling),c=i(r,a.props.children),c.return=e,e=c;break a}}else if(r.elementType===l||typeof l==`object`&&l&&l.$$typeof===ne&&Pa(l)===r.type){n(e,r.sibling),c=i(r,a.props),Va(c,a),c.return=e,e=c;break a}n(e,r);break}else t(e,r);r=r.sibling}a.type===y?(c=bi(a.props.children,e.mode,c,a.key),c.return=e,e=c):(c=yi(a.type,a.key,a.props,null,e.mode,c),Va(c,a),c.return=e,e=c)}return o(e);case v:a:{for(l=a.key;r!==null;){if(r.key===l)if(r.tag===4&&r.stateNode.containerInfo===a.containerInfo&&r.stateNode.implementation===a.implementation){n(e,r.sibling),c=i(r,a.children||[]),c.return=e,e=c;break a}else{n(e,r);break}else t(e,r);r=r.sibling}c=Ci(a,e.mode,c),c.return=e,e=c}return o(e);case ne:return a=Pa(a),b(e,r,a,c)}if(ce(a))return h(e,r,a,c);if(ae(a)){if(l=ae(a),typeof l!=`function`)throw Error(s(150));return a=l.call(a),g(e,r,a,c)}if(typeof a.then==`function`)return b(e,r,Ba(a),c);if(a.$$typeof===C)return b(e,r,ca(e,a),c);Ha(e,a)}return typeof a==`string`&&a!==``||typeof a==`number`||typeof a==`bigint`?(a=``+a,r!==null&&r.tag===6?(n(e,r.sibling),c=i(r,a),c.return=e,e=c):(n(e,r),c=xi(a,e.mode,c),c.return=e,e=c),o(e)):n(e,r)}return function(e,t,n,r){try{za=0;var i=b(e,t,n,r);return Ra=null,i}catch(t){if(t===Oa||t===Aa)throw t;var a=hi(29,t,null,e.mode);return a.lanes=r,a.return=e,a}}}var Wa=Ua(!0),Ga=Ua(!1),Ka=!1;function qa(e){e.updateQueue={baseState:e.memoizedState,firstBaseUpdate:null,lastBaseUpdate:null,shared:{pending:null,lanes:0,hiddenCallbacks:null},callbacks:null}}function Ja(e,t){e=e.updateQueue,t.updateQueue===e&&(t.updateQueue={baseState:e.baseState,firstBaseUpdate:e.firstBaseUpdate,lastBaseUpdate:e.lastBaseUpdate,shared:e.shared,callbacks:null})}function Ya(e){return{lane:e,tag:0,payload:null,callback:null,next:null}}function Xa(e,t,n){var r=e.updateQueue;if(r===null)return null;if(r=r.shared,B&2){var i=r.pending;return i===null?t.next=t:(t.next=i.next,i.next=t),r.pending=t,t=fi(e),di(e,null,n),t}return ci(e,r,t,n),fi(e)}function Za(e,t,n){if(t=t.updateQueue,t!==null&&(t=t.shared,n&4194048)){var r=t.lanes;r&=e.pendingLanes,n|=r,t.lanes=n,lt(e,n)}}function Qa(e,t){var n=e.updateQueue,r=e.alternate;if(r!==null&&(r=r.updateQueue,n===r)){var i=null,a=null;if(n=n.firstBaseUpdate,n!==null){do{var o={lane:n.lane,tag:n.tag,payload:n.payload,callback:null,next:null};a===null?i=a=o:a=a.next=o,n=n.next}while(n!==null);a===null?i=a=t:a=a.next=t}else i=a=t;n={baseState:r.baseState,firstBaseUpdate:i,lastBaseUpdate:a,shared:r.shared,callbacks:r.callbacks},e.updateQueue=n;return}e=n.lastBaseUpdate,e===null?n.firstBaseUpdate=t:e.next=t,n.lastBaseUpdate=t}var $a=!1;function eo(){if($a){var e=ya;if(e!==null)throw e}}function to(e,t,n,r){$a=!1;var i=e.updateQueue;Ka=!1;var a=i.firstBaseUpdate,o=i.lastBaseUpdate,s=i.shared.pending;if(s!==null){i.shared.pending=null;var c=s,l=c.next;c.next=null,o===null?a=l:o.next=l,o=c;var u=e.alternate;u!==null&&(u=u.updateQueue,s=u.lastBaseUpdate,s!==o&&(s===null?u.firstBaseUpdate=l:s.next=l,u.lastBaseUpdate=c))}if(a!==null){var d=i.baseState;o=0,u=l=c=null,s=a;do{var f=s.lane&-536870913,p=f!==s.lane;if(p?(H&f)===f:(r&f)===f){f!==0&&f===va&&($a=!0),u!==null&&(u=u.next={lane:0,tag:s.tag,payload:s.payload,callback:null,next:null});a:{var m=e,g=s;f=t;var _=n;switch(g.tag){case 1:if(m=g.payload,typeof m==`function`){d=m.call(_,d,f);break a}d=m;break a;case 3:m.flags=m.flags&-65537|128;case 0:if(m=g.payload,f=typeof m==`function`?m.call(_,d,f):m,f==null)break a;d=h({},d,f);break a;case 2:Ka=!0}}f=s.callback,f!==null&&(e.flags|=64,p&&(e.flags|=8192),p=i.callbacks,p===null?i.callbacks=[f]:p.push(f))}else p={lane:f,tag:s.tag,payload:s.payload,callback:s.callback,next:null},u===null?(l=u=p,c=d):u=u.next=p,o|=f;if(s=s.next,s===null){if(s=i.shared.pending,s===null)break;p=s,s=p.next,p.next=null,i.lastBaseUpdate=p,i.shared.pending=null}}while(1);u===null&&(c=d),i.baseState=c,i.firstBaseUpdate=l,i.lastBaseUpdate=u,a===null&&(i.shared.lanes=0),Zl|=o,e.lanes=o,e.memoizedState=d}}function no(e,t){if(typeof e!=`function`)throw Error(s(191,e));e.call(t)}function ro(e,t){var n=e.callbacks;if(n!==null)for(e.callbacks=null,e=0;e<n.length;e++)no(n[e],t)}var io=fe(null),ao=fe(0);function oo(e,t){e=U,me(ao,e),me(io,t),U=e|t.baseLanes}function so(){me(ao,U),me(io,io.current)}function co(){U=ao.current,pe(io),pe(ao)}var lo=fe(null),uo=null;function fo(e){var t=e.alternate;me(go,go.current&1),me(lo,e),uo===null&&(t===null||io.current!==null||t.memoizedState!==null)&&(uo=e)}function P(e){me(go,go.current),me(lo,e),uo===null&&(uo=e)}function po(e){e.tag===22?(me(go,go.current),me(lo,e),uo===null&&(uo=e)):mo(e)}function mo(){me(go,go.current),me(lo,lo.current)}function ho(e){pe(lo),uo===e&&(uo=null),pe(go)}var go=fe(0);function _o(e){for(var t=e;t!==null;){if(t.tag===13){var n=t.memoizedState;if(n!==null&&(n=n.dehydrated,n===null||cf(n)||lf(n)))return t}else if(t.tag===19&&(t.memoizedProps.revealOrder===`forwards`||t.memoizedProps.revealOrder===`backwards`||t.memoizedProps.revealOrder===`unstable_legacy-backwards`||t.memoizedProps.revealOrder===`together`)){if(t.flags&128)return t}else if(t.child!==null){t.child.return=t,t=t.child;continue}if(t===e)break;for(;t.sibling===null;){if(t.return===null||t.return===e)return null;t=t.return}t.sibling.return=t.return,t=t.sibling}return null}var vo=0,F=null,yo=null,bo=null,xo=!1,So=!1,Co=!1,wo=0,To=0,Eo=null,Do=0;function Oo(){throw Error(s(321))}function ko(e,t){if(t===null)return!1;for(var n=0;n<t.length&&n<e.length;n++)if(!Ar(e[n],t[n]))return!1;return!0}function Ao(e,t,n,r,i,a){return vo=a,F=t,t.memoizedState=null,t.updateQueue=null,t.lanes=0,D.H=e===null||e.memoizedState===null?Gs:Ks,Co=!1,a=n(r,i),Co=!1,So&&(a=Mo(t,n,r,i)),jo(e),a}function jo(e){D.H=Ws;var t=yo!==null&&yo.next!==null;if(vo=0,bo=yo=F=null,xo=!1,To=0,Eo=null,t)throw Error(s(300));e===null||lc||(e=e.dependencies,e!==null&&aa(e)&&(lc=!0))}function Mo(e,t,n,r){F=e;var i=0;do{if(So&&(Eo=null),To=0,So=!1,25<=i)throw Error(s(301));if(i+=1,bo=yo=null,e.updateQueue!=null){var a=e.updateQueue;a.lastEffect=null,a.events=null,a.stores=null,a.memoCache!=null&&(a.memoCache.index=0)}D.H=qs,a=t(n,r)}while(So);return a}function No(){var e=D.H,t=e.useState()[0];return t=typeof t.then==`function`?Bo(t):t,e=e.useState()[0],(yo===null?null:yo.memoizedState)!==e&&(F.flags|=1024),t}function Po(){var e=wo!==0;return wo=0,e}function Fo(e,t,n){t.updateQueue=e.updateQueue,t.flags&=-2053,e.lanes&=~n}function Io(e){if(xo){for(e=e.memoizedState;e!==null;){var t=e.queue;t!==null&&(t.pending=null),e=e.next}xo=!1}vo=0,bo=yo=F=null,So=!1,To=wo=0,Eo=null}function Lo(){var e={memoizedState:null,baseState:null,baseQueue:null,queue:null,next:null};return bo===null?F.memoizedState=bo=e:bo=bo.next=e,bo}function Ro(){if(yo===null){var e=F.alternate;e=e===null?null:e.memoizedState}else e=yo.next;var t=bo===null?F.memoizedState:bo.next;if(t!==null)bo=t,yo=e;else{if(e===null)throw F.alternate===null?Error(s(467)):Error(s(310));yo=e,e={memoizedState:yo.memoizedState,baseState:yo.baseState,baseQueue:yo.baseQueue,queue:yo.queue,next:null},bo===null?F.memoizedState=bo=e:bo=bo.next=e}return bo}function zo(){return{lastEffect:null,events:null,stores:null,memoCache:null}}function Bo(e){var t=To;return To+=1,Eo===null&&(Eo=[]),e=Na(Eo,e,t),t=F,(bo===null?t.memoizedState:bo.next)===null&&(t=t.alternate,D.H=t===null||t.memoizedState===null?Gs:Ks),e}function Vo(e){if(typeof e==`object`&&e){if(typeof e.then==`function`)return Bo(e);if(e.$$typeof===C)return sa(e)}throw Error(s(438,String(e)))}function Ho(e){var t=null,n=F.updateQueue;if(n!==null&&(t=n.memoCache),t==null){var r=F.alternate;r!==null&&(r=r.updateQueue,r!==null&&(r=r.memoCache,r!=null&&(t={data:r.data.map(function(e){return e.slice()}),index:0})))}if(t??={data:[],index:0},n===null&&(n=zo(),F.updateQueue=n),n.memoCache=t,n=t.data[t.index],n===void 0)for(n=t.data[t.index]=Array(e),r=0;r<e;r++)n[r]=ie;return t.index++,n}function Uo(e,t){return typeof t==`function`?t(e):t}function Wo(e){return Go(Ro(),yo,e)}function Go(e,t,n){var r=e.queue;if(r===null)throw Error(s(311));r.lastRenderedReducer=n;var i=e.baseQueue,a=r.pending;if(a!==null){if(i!==null){var o=i.next;i.next=a.next,a.next=o}t.baseQueue=i=a,r.pending=null}if(a=e.baseState,i===null)e.memoizedState=a;else{t=i.next;var c=o=null,l=null,u=t,d=!1;do{var f=u.lane&-536870913;if(f===u.lane?(vo&f)===f:(H&f)===f){var p=u.revertLane;if(p===0)l!==null&&(l=l.next={lane:0,revertLane:0,gesture:null,action:u.action,hasEagerState:u.hasEagerState,eagerState:u.eagerState,next:null}),f===va&&(d=!0);else if((vo&p)===p){u=u.next,p===va&&(d=!0);continue}else f={lane:0,revertLane:u.revertLane,gesture:null,action:u.action,hasEagerState:u.hasEagerState,eagerState:u.eagerState,next:null},l===null?(c=l=f,o=a):l=l.next=f,F.lanes|=p,Zl|=p;f=u.action,Co&&n(a,f),a=u.hasEagerState?u.eagerState:n(a,f)}else p={lane:f,revertLane:u.revertLane,gesture:u.gesture,action:u.action,hasEagerState:u.hasEagerState,eagerState:u.eagerState,next:null},l===null?(c=l=p,o=a):l=l.next=p,F.lanes|=f,Zl|=f;u=u.next}while(u!==null&&u!==t);if(l===null?o=a:l.next=c,!Ar(a,e.memoizedState)&&(lc=!0,d&&(n=ya,n!==null)))throw n;e.memoizedState=a,e.baseState=o,e.baseQueue=l,r.lastRenderedState=a}return i===null&&(r.lanes=0),[e.memoizedState,r.dispatch]}function Ko(e){var t=Ro(),n=t.queue;if(n===null)throw Error(s(311));n.lastRenderedReducer=e;var r=n.dispatch,i=n.pending,a=t.memoizedState;if(i!==null){n.pending=null;var o=i=i.next;do a=e(a,o.action),o=o.next;while(o!==i);Ar(a,t.memoizedState)||(lc=!0),t.memoizedState=a,t.baseQueue===null&&(t.baseState=a),n.lastRenderedState=a}return[a,r]}function qo(e,t,n){var r=F,i=Ro(),a=M;if(a){if(n===void 0)throw Error(s(407));n=n()}else n=t();var o=!Ar((yo||i).memoizedState,n);if(o&&(i.memoizedState=n,lc=!0),i=i.queue,_s(Xo.bind(null,r,i,e),[e]),i.getSnapshot!==t||o||bo!==null&&bo.memoizedState.tag&1){if(r.flags|=2048,ps(9,{destroy:void 0},Yo.bind(null,r,i,n,t),null),Gl===null)throw Error(s(349));a||vo&127||Jo(r,t,n)}return n}function Jo(e,t,n){e.flags|=16384,e={getSnapshot:t,value:n},t=F.updateQueue,t===null?(t=zo(),F.updateQueue=t,t.stores=[e]):(n=t.stores,n===null?t.stores=[e]:n.push(e))}function Yo(e,t,n,r){t.value=n,t.getSnapshot=r,Zo(t)&&Qo(e)}function Xo(e,t,n){return n(function(){Zo(t)&&Qo(e)})}function Zo(e){var t=e.getSnapshot;e=e.value;try{var n=t();return!Ar(e,n)}catch{return!0}}function Qo(e){var t=ui(e,2);t!==null&&K(t,e,2)}function $o(e){var t=Lo();if(typeof e==`function`){var n=e;if(e=n(),Co){Ke(!0);try{n()}finally{Ke(!1)}}}return t.memoizedState=t.baseState=e,t.queue={pending:null,lanes:0,dispatch:null,lastRenderedReducer:Uo,lastRenderedState:e},t}function es(e,t,n,r){return e.baseState=n,Go(e,yo,typeof r==`function`?r:Uo)}function ts(e,t,n,r,i){if(Vs(e))throw Error(s(485));if(e=t.action,e!==null){var a={payload:i,action:e,next:null,isTransition:!0,status:`pending`,value:null,reason:null,listeners:[],then:function(e){a.listeners.push(e)}};D.T===null?a.isTransition=!1:n(!0),r(a),n=t.pending,n===null?(a.next=t.pending=a,ns(t,a)):(a.next=n.next,t.pending=n.next=a)}}function ns(e,t){var n=t.action,r=t.payload,i=e.state;if(t.isTransition){var a=D.T,o={};D.T=o;try{var s=n(i,r),c=D.S;c!==null&&c(o,s),rs(e,t,s)}catch(n){as(e,t,n)}finally{a!==null&&o.types!==null&&(a.types=o.types),D.T=a}}else try{a=n(i,r),rs(e,t,a)}catch(n){as(e,t,n)}}function rs(e,t,n){typeof n==`object`&&n&&typeof n.then==`function`?n.then(function(n){is(e,t,n)},function(n){return as(e,t,n)}):is(e,t,n)}function is(e,t,n){t.status=`fulfilled`,t.value=n,os(t),e.state=n,t=e.pending,t!==null&&(n=t.next,n===t?e.pending=null:(n=n.next,t.next=n,ns(e,n)))}function as(e,t,n){var r=e.pending;if(e.pending=null,r!==null){r=r.next;do t.status=`rejected`,t.reason=n,os(t),t=t.next;while(t!==r)}e.action=null}function os(e){e=e.listeners;for(var t=0;t<e.length;t++)(0,e[t])()}function ss(e,t){return t}function cs(e,t){if(M){var n=Gl.formState;if(n!==null){a:{var r=F;if(M){if(Vi){b:{for(var i=Vi,a=Ui;i.nodeType!==8;){if(!a){i=null;break b}if(i=df(i.nextSibling),i===null){i=null;break b}}a=i.data,i=a===`F!`||a===`F`?i:null}if(i){Vi=df(i.nextSibling),r=i.data===`F!`;break a}}Gi(r)}r=!1}r&&(t=n[0])}}return n=Lo(),n.memoizedState=n.baseState=t,r={pending:null,lanes:0,dispatch:null,lastRenderedReducer:ss,lastRenderedState:t},n.queue=r,n=Rs.bind(null,F,r),r.dispatch=n,r=$o(!1),a=Bs.bind(null,F,!1,r.queue),r=Lo(),i={state:t,dispatch:null,action:e,pending:null},r.queue=i,n=ts.bind(null,F,i,a,n),i.dispatch=n,r.memoizedState=e,[t,n,!1]}function ls(e){return us(Ro(),yo,e)}function us(e,t,n){if(t=Go(e,t,ss)[0],e=Wo(Uo)[0],typeof t==`object`&&t&&typeof t.then==`function`)try{var r=Bo(t)}catch(e){throw e===Oa?Aa:e}else r=t;t=Ro();var i=t.queue,a=i.dispatch;return n!==t.memoizedState&&(F.flags|=2048,ps(9,{destroy:void 0},ds.bind(null,i,n),null)),[r,a,e]}function ds(e,t){e.action=t}function fs(e){var t=Ro(),n=yo;if(n!==null)return us(t,n,e);Ro(),t=t.memoizedState,n=Ro();var r=n.queue.dispatch;return n.memoizedState=e,[t,r,!1]}function ps(e,t,n,r){return e={tag:e,create:n,deps:r,inst:t,next:null},t=F.updateQueue,t===null&&(t=zo(),F.updateQueue=t),n=t.lastEffect,n===null?t.lastEffect=e.next=e:(r=n.next,n.next=e,e.next=r,t.lastEffect=e),e}function ms(){return Ro().memoizedState}function hs(e,t,n,r){var i=Lo();F.flags|=e,i.memoizedState=ps(1|t,{destroy:void 0},n,r===void 0?null:r)}function I(e,t,n,r){var i=Ro();r=r===void 0?null:r;var a=i.memoizedState.inst;yo!==null&&r!==null&&ko(r,yo.memoizedState.deps)?i.memoizedState=ps(t,a,n,r):(F.flags|=e,i.memoizedState=ps(1|t,a,n,r))}function gs(e,t){hs(8390656,8,e,t)}function _s(e,t){I(2048,8,e,t)}function L(e){F.flags|=4;var t=F.updateQueue;if(t===null)t=zo(),F.updateQueue=t,t.events=[e];else{var n=t.events;n===null?t.events=[e]:n.push(e)}}function vs(e){var t=Ro().memoizedState;return L({ref:t,nextImpl:e}),function(){if(B&2)throw Error(s(440));return t.impl.apply(void 0,arguments)}}function ys(e,t){return I(4,2,e,t)}function bs(e,t){return I(4,4,e,t)}function xs(e,t){if(typeof t==`function`){e=e();var n=t(e);return function(){typeof n==`function`?n():t(null)}}if(t!=null)return e=e(),t.current=e,function(){t.current=null}}function Ss(e,t,n){n=n==null?null:n.concat([e]),I(4,4,xs.bind(null,t,e),n)}function Cs(){}function ws(e,t){var n=Ro();t=t===void 0?null:t;var r=n.memoizedState;return t!==null&&ko(t,r[1])?r[0]:(n.memoizedState=[e,t],e)}function Ts(e,t){var n=Ro();t=t===void 0?null:t;var r=n.memoizedState;if(t!==null&&ko(t,r[1]))return r[0];if(r=e(),Co){Ke(!0);try{e()}finally{Ke(!1)}}return n.memoizedState=[r,t],r}function Es(e,t,n){return n===void 0||vo&1073741824&&!(H&261930)?e.memoizedState=t:(e.memoizedState=n,e=yu(),F.lanes|=e,Zl|=e,n)}function Ds(e,t,n,r){return Ar(n,t)?n:io.current===null?!(vo&42)||vo&1073741824&&!(H&261930)?(lc=!0,e.memoizedState=n):(e=yu(),F.lanes|=e,Zl|=e,t):(e=Es(e,n,r),Ar(e,t)||(lc=!0),e)}function Os(e,t,n,r,i){var a=O.p;O.p=a!==0&&8>a?a:8;var o=D.T,s={};D.T=s,Bs(e,!1,t,n);try{var c=i(),l=D.S;l!==null&&l(s,c),typeof c==`object`&&c&&typeof c.then==`function`?zs(e,t,Sa(c,r),vu(e)):zs(e,t,r,vu(e))}catch(n){zs(e,t,{then:function(){},status:`rejected`,reason:n},vu())}finally{O.p=a,o!==null&&s.types!==null&&(o.types=s.types),D.T=o}}function ks(){}function As(e,t,n,r){if(e.tag!==5)throw Error(s(476));var i=js(e).queue;Os(e,i,t,le,n===null?ks:function(){return Ms(e),n(r)})}function js(e){var t=e.memoizedState;if(t!==null)return t;t={memoizedState:le,baseState:le,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:Uo,lastRenderedState:le},next:null};var n={};return t.next={memoizedState:n,baseState:n,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:Uo,lastRenderedState:n},next:null},e.memoizedState=t,e=e.alternate,e!==null&&(e.memoizedState=t),t}function Ms(e){var t=js(e);t.next===null&&(t=e.alternate.memoizedState),zs(e,t.next.queue,{},vu())}function Ns(){return sa(ep)}function Ps(){return Ro().memoizedState}function Fs(){return Ro().memoizedState}function Is(e){for(var t=e.return;t!==null;){switch(t.tag){case 24:case 3:var n=vu();e=Ya(n);var r=Xa(t,e,n);r!==null&&(K(r,t,n),Za(r,t,n)),t={cache:ma()},e.payload=t;return}t=t.return}}function Ls(e,t,n){var r=vu();n={lane:r,revertLane:0,gesture:null,action:n,hasEagerState:!1,eagerState:null,next:null},Vs(e)?Hs(t,n):(n=li(e,t,n,r),n!==null&&(K(n,e,r),Us(n,t,r)))}function Rs(e,t,n){zs(e,t,n,vu())}function zs(e,t,n,r){var i={lane:r,revertLane:0,gesture:null,action:n,hasEagerState:!1,eagerState:null,next:null};if(Vs(e))Hs(t,i);else{var a=e.alternate;if(e.lanes===0&&(a===null||a.lanes===0)&&(a=t.lastRenderedReducer,a!==null))try{var o=t.lastRenderedState,s=a(o,n);if(i.hasEagerState=!0,i.eagerState=s,Ar(s,o))return ci(e,t,i,0),Gl===null&&si(),!1}catch{}if(n=li(e,t,i,r),n!==null)return K(n,e,r),Us(n,t,r),!0}return!1}function Bs(e,t,n,r){if(r={lane:2,revertLane:md(),gesture:null,action:r,hasEagerState:!1,eagerState:null,next:null},Vs(e)){if(t)throw Error(s(479))}else t=li(e,n,r,2),t!==null&&K(t,e,2)}function Vs(e){var t=e.alternate;return e===F||t!==null&&t===F}function Hs(e,t){So=xo=!0;var n=e.pending;n===null?t.next=t:(t.next=n.next,n.next=t),e.pending=t}function Us(e,t,n){if(n&4194048){var r=t.lanes;r&=e.pendingLanes,n|=r,t.lanes=n,lt(e,n)}}var Ws={readContext:sa,use:Vo,useCallback:Oo,useContext:Oo,useEffect:Oo,useImperativeHandle:Oo,useLayoutEffect:Oo,useInsertionEffect:Oo,useMemo:Oo,useReducer:Oo,useRef:Oo,useState:Oo,useDebugValue:Oo,useDeferredValue:Oo,useTransition:Oo,useSyncExternalStore:Oo,useId:Oo,useHostTransitionStatus:Oo,useFormState:Oo,useActionState:Oo,useOptimistic:Oo,useMemoCache:Oo,useCacheRefresh:Oo};Ws.useEffectEvent=Oo;var Gs={readContext:sa,use:Vo,useCallback:function(e,t){return Lo().memoizedState=[e,t===void 0?null:t],e},useContext:sa,useEffect:gs,useImperativeHandle:function(e,t,n){n=n==null?null:n.concat([e]),hs(4194308,4,xs.bind(null,t,e),n)},useLayoutEffect:function(e,t){return hs(4194308,4,e,t)},useInsertionEffect:function(e,t){hs(4,2,e,t)},useMemo:function(e,t){var n=Lo();t=t===void 0?null:t;var r=e();if(Co){Ke(!0);try{e()}finally{Ke(!1)}}return n.memoizedState=[r,t],r},useReducer:function(e,t,n){var r=Lo();if(n!==void 0){var i=n(t);if(Co){Ke(!0);try{n(t)}finally{Ke(!1)}}}else i=t;return r.memoizedState=r.baseState=i,e={pending:null,lanes:0,dispatch:null,lastRenderedReducer:e,lastRenderedState:i},r.queue=e,e=e.dispatch=Ls.bind(null,F,e),[r.memoizedState,e]},useRef:function(e){var t=Lo();return e={current:e},t.memoizedState=e},useState:function(e){e=$o(e);var t=e.queue,n=Rs.bind(null,F,t);return t.dispatch=n,[e.memoizedState,n]},useDebugValue:Cs,useDeferredValue:function(e,t){return Es(Lo(),e,t)},useTransition:function(){var e=$o(!1);return e=Os.bind(null,F,e.queue,!0,!1),Lo().memoizedState=e,[!1,e]},useSyncExternalStore:function(e,t,n){var r=F,i=Lo();if(M){if(n===void 0)throw Error(s(407));n=n()}else{if(n=t(),Gl===null)throw Error(s(349));H&127||Jo(r,t,n)}i.memoizedState=n;var a={value:n,getSnapshot:t};return i.queue=a,gs(Xo.bind(null,r,a,e),[e]),r.flags|=2048,ps(9,{destroy:void 0},Yo.bind(null,r,a,n,t),null),n},useId:function(){var e=Lo(),t=Gl.identifierPrefix;if(M){var n=Pi,r=Ni;n=(r&~(1<<32-qe(r)-1)).toString(32)+n,t=`_`+t+`R_`+n,n=wo++,0<n&&(t+=`H`+n.toString(32)),t+=`_`}else n=Do++,t=`_`+t+`r_`+n.toString(32)+`_`;return e.memoizedState=t},useHostTransitionStatus:Ns,useFormState:cs,useActionState:cs,useOptimistic:function(e){var t=Lo();t.memoizedState=t.baseState=e;var n={pending:null,lanes:0,dispatch:null,lastRenderedReducer:null,lastRenderedState:null};return t.queue=n,t=Bs.bind(null,F,!0,n),n.dispatch=t,[e,t]},useMemoCache:Ho,useCacheRefresh:function(){return Lo().memoizedState=Is.bind(null,F)},useEffectEvent:function(e){var t=Lo(),n={impl:e};return t.memoizedState=n,function(){if(B&2)throw Error(s(440));return n.impl.apply(void 0,arguments)}}},Ks={readContext:sa,use:Vo,useCallback:ws,useContext:sa,useEffect:_s,useImperativeHandle:Ss,useInsertionEffect:ys,useLayoutEffect:bs,useMemo:Ts,useReducer:Wo,useRef:ms,useState:function(){return Wo(Uo)},useDebugValue:Cs,useDeferredValue:function(e,t){return Ds(Ro(),yo.memoizedState,e,t)},useTransition:function(){var e=Wo(Uo)[0],t=Ro().memoizedState;return[typeof e==`boolean`?e:Bo(e),t]},useSyncExternalStore:qo,useId:Ps,useHostTransitionStatus:Ns,useFormState:ls,useActionState:ls,useOptimistic:function(e,t){return es(Ro(),yo,e,t)},useMemoCache:Ho,useCacheRefresh:Fs};Ks.useEffectEvent=vs;var qs={readContext:sa,use:Vo,useCallback:ws,useContext:sa,useEffect:_s,useImperativeHandle:Ss,useInsertionEffect:ys,useLayoutEffect:bs,useMemo:Ts,useReducer:Ko,useRef:ms,useState:function(){return Ko(Uo)},useDebugValue:Cs,useDeferredValue:function(e,t){var n=Ro();return yo===null?Es(n,e,t):Ds(n,yo.memoizedState,e,t)},useTransition:function(){var e=Ko(Uo)[0],t=Ro().memoizedState;return[typeof e==`boolean`?e:Bo(e),t]},useSyncExternalStore:qo,useId:Ps,useHostTransitionStatus:Ns,useFormState:fs,useActionState:fs,useOptimistic:function(e,t){var n=Ro();return yo===null?(n.baseState=e,[e,n.queue.dispatch]):es(n,yo,e,t)},useMemoCache:Ho,useCacheRefresh:Fs};qs.useEffectEvent=vs;function Js(e,t,n,r){t=e.memoizedState,n=n(r,t),n=n==null?t:h({},t,n),e.memoizedState=n,e.lanes===0&&(e.updateQueue.baseState=n)}var Ys={enqueueSetState:function(e,t,n){e=e._reactInternals;var r=vu(),i=Ya(r);i.payload=t,n!=null&&(i.callback=n),t=Xa(e,i,r),t!==null&&(K(t,e,r),Za(t,e,r))},enqueueReplaceState:function(e,t,n){e=e._reactInternals;var r=vu(),i=Ya(r);i.tag=1,i.payload=t,n!=null&&(i.callback=n),t=Xa(e,i,r),t!==null&&(K(t,e,r),Za(t,e,r))},enqueueForceUpdate:function(e,t){e=e._reactInternals;var n=vu(),r=Ya(n);r.tag=2,t!=null&&(r.callback=t),t=Xa(e,r,n),t!==null&&(K(t,e,n),Za(t,e,n))}};function Xs(e,t,n,r,i,a,o){return e=e.stateNode,typeof e.shouldComponentUpdate==`function`?e.shouldComponentUpdate(r,a,o):t.prototype&&t.prototype.isPureReactComponent?!jr(n,r)||!jr(i,a):!0}function Zs(e,t,n,r){e=t.state,typeof t.componentWillReceiveProps==`function`&&t.componentWillReceiveProps(n,r),typeof t.UNSAFE_componentWillReceiveProps==`function`&&t.UNSAFE_componentWillReceiveProps(n,r),t.state!==e&&Ys.enqueueReplaceState(t,t.state,null)}function Qs(e,t){var n=t;if(`ref`in t)for(var r in n={},t)r!==`ref`&&(n[r]=t[r]);if(e=e.defaultProps)for(var i in n===t&&(n=h({},n)),e)n[i]===void 0&&(n[i]=e[i]);return n}function $s(e){ri(e)}function ec(e){console.error(e)}function tc(e){ri(e)}function nc(e,t){try{var n=e.onUncaughtError;n(t.value,{componentStack:t.stack})}catch(e){setTimeout(function(){throw e})}}function rc(e,t,n){try{var r=e.onCaughtError;r(n.value,{componentStack:n.stack,errorBoundary:t.tag===1?t.stateNode:null})}catch(e){setTimeout(function(){throw e})}}function ic(e,t,n){return n=Ya(n),n.tag=3,n.payload={element:null},n.callback=function(){nc(e,t)},n}function ac(e){return e=Ya(e),e.tag=3,e}function oc(e,t,n,r){var i=n.type.getDerivedStateFromError;if(typeof i==`function`){var a=r.value;e.payload=function(){return i(a)},e.callback=function(){rc(t,n,r)}}var o=n.stateNode;o!==null&&typeof o.componentDidCatch==`function`&&(e.callback=function(){rc(t,n,r),typeof i!=`function`&&(G===null?G=new Set([this]):G.add(this));var e=r.stack;this.componentDidCatch(r.value,{componentStack:e===null?``:e})})}function sc(e,t,n,r,i){if(n.flags|=32768,typeof r==`object`&&r&&typeof r.then==`function`){if(t=n.alternate,t!==null&&ia(t,n,i,!0),n=lo.current,n!==null){switch(n.tag){case 31:case 13:return uo===null?ju():n.alternate===null&&W===0&&(W=3),n.flags&=-257,n.flags|=65536,n.lanes=i,r===ja?n.flags|=16384:(t=n.updateQueue,t===null?n.updateQueue=new Set([r]):t.add(r),Ju(e,r,i)),!1;case 22:return n.flags|=65536,r===ja?n.flags|=16384:(t=n.updateQueue,t===null?(t={transitions:null,markerInstances:null,retryQueue:new Set([r])},n.updateQueue=t):(n=t.retryQueue,n===null?t.retryQueue=new Set([r]):n.add(r)),Ju(e,r,i)),!1}throw Error(s(435,n.tag))}return Ju(e,r,i),ju(),!1}if(M)return t=lo.current,t===null?(r!==Wi&&(t=Error(s(423),{cause:r}),Zi(Ti(t,n))),e=e.current.alternate,e.flags|=65536,i&=-i,e.lanes|=i,r=Ti(r,n),i=ic(e.stateNode,r,i),Qa(e,i),W!==4&&(W=2)):(!(t.flags&65536)&&(t.flags|=256),t.flags|=65536,t.lanes=i,r!==Wi&&(e=Error(s(422),{cause:r}),Zi(Ti(e,n)))),!1;var a=Error(s(520),{cause:r});if(a=Ti(a,n),nu===null?nu=[a]:nu.push(a),W!==4&&(W=2),t===null)return!0;r=Ti(r,n),n=t;do{switch(n.tag){case 3:return n.flags|=65536,e=i&-i,n.lanes|=e,e=ic(n.stateNode,r,e),Qa(n,e),!1;case 1:if(t=n.type,a=n.stateNode,!(n.flags&128)&&(typeof t.getDerivedStateFromError==`function`||a!==null&&typeof a.componentDidCatch==`function`&&(G===null||!G.has(a))))return n.flags|=65536,i&=-i,n.lanes|=i,i=ac(i),oc(i,e,n,r),Qa(n,i),!1}n=n.return}while(n!==null);return!1}var cc=Error(s(461)),lc=!1;function uc(e,t,n,r){t.child=e===null?Ga(t,null,n,r):Wa(t,e.child,n,r)}function dc(e,t,n,r,i){n=n.render;var a=t.ref;if(`ref`in r){var o={};for(var s in r)s!==`ref`&&(o[s]=r[s])}else o=r;return oa(t),r=Ao(e,t,n,o,a,i),s=Po(),e!==null&&!lc?(Fo(e,t,i),Fc(e,t,i)):(M&&s&&Li(t),t.flags|=1,uc(e,t,r,i),t.child)}function fc(e,t,n,r,i){if(e===null){var a=n.type;return typeof a==`function`&&!gi(a)&&a.defaultProps===void 0&&n.compare===null?(t.tag=15,t.type=a,pc(e,t,a,r,i)):(e=yi(n.type,null,r,t,t.mode,i),e.ref=t.ref,e.return=t,t.child=e)}if(a=e.child,!Ic(e,i)){var o=a.memoizedProps;if(n=n.compare,n=n===null?jr:n,n(o,r)&&e.ref===t.ref)return Fc(e,t,i)}return t.flags|=1,e=_i(a,r),e.ref=t.ref,e.return=t,t.child=e}function pc(e,t,n,r,i){if(e!==null){var a=e.memoizedProps;if(jr(a,r)&&e.ref===t.ref)if(lc=!1,t.pendingProps=r=a,Ic(e,i))e.flags&131072&&(lc=!0);else return t.lanes=e.lanes,Fc(e,t,i)}return xc(e,t,n,r,i)}function mc(e,t,n,r){var i=r.children,a=e===null?null:e.memoizedState;if(e===null&&t.stateNode===null&&(t.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),r.mode===`hidden`){if(t.flags&128){if(a=a===null?n:a.baseLanes|n,e!==null){for(r=t.child=e.child,i=0;r!==null;)i=i|r.lanes|r.childLanes,r=r.sibling;r=i&~a}else r=0,t.child=null;return gc(e,t,a,n,r)}if(n&536870912)t.memoizedState={baseLanes:0,cachePool:null},e!==null&&Ea(t,a===null?null:a.cachePool),a===null?so():oo(t,a),po(t);else return r=t.lanes=536870912,gc(e,t,a===null?n:a.baseLanes|n,n,r)}else a===null?(e!==null&&Ea(t,null),so(),mo(t)):(Ea(t,a.cachePool),oo(t,a),mo(t),t.memoizedState=null);return uc(e,t,i,n),t.child}function hc(e,t){return e!==null&&e.tag===22||t.stateNode!==null||(t.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),t.sibling}function gc(e,t,n,r,i){var a=Ta();return a=a===null?null:{parent:pa._currentValue,pool:a},t.memoizedState={baseLanes:n,cachePool:a},e!==null&&Ea(t,null),so(),po(t),e!==null&&ia(e,t,r,!0),t.childLanes=i,null}function _c(e,t){return t=Ac({mode:t.mode,children:t.children},e.mode),t.ref=e.ref,e.child=t,t.return=e,t}function vc(e,t,n){return Wa(t,e.child,null,n),e=_c(t,t.pendingProps),e.flags|=2,ho(t),t.memoizedState=null,e}function yc(e,t,n){var r=t.pendingProps,i=(t.flags&128)!=0;if(t.flags&=-129,e===null){if(M){if(r.mode===`hidden`)return e=_c(t,r),t.lanes=536870912,hc(null,e);if(P(t),(e=Vi)?(e=sf(e,Ui),e=e!==null&&e.data===`&`?e:null,e!==null&&(t.memoizedState={dehydrated:e,treeContext:Mi===null?null:{id:Ni,overflow:Pi},retryLane:536870912,hydrationErrors:null},n=Si(e),n.return=t,t.child=n,Bi=t,Vi=null)):e=null,e===null)throw Gi(t);return t.lanes=536870912,null}return _c(t,r)}var a=e.memoizedState;if(a!==null){var o=a.dehydrated;if(P(t),i)if(t.flags&256)t.flags&=-257,t=vc(e,t,n);else if(t.memoizedState!==null)t.child=e.child,t.flags|=128,t=null;else throw Error(s(558));else if(lc||ia(e,t,n,!1),i=(n&e.childLanes)!==0,lc||i){if(r=Gl,r!==null&&(o=ut(r,n),o!==0&&o!==a.retryLane))throw a.retryLane=o,ui(e,o),K(r,e,o),cc;ju(),t=vc(e,t,n)}else e=a.treeContext,Vi=df(o.nextSibling),Bi=t,M=!0,Hi=null,Ui=!1,e!==null&&zi(t,e),t=_c(t,r),t.flags|=4096;return t}return e=_i(e.child,{mode:r.mode,children:r.children}),e.ref=t.ref,t.child=e,e.return=t,e}function bc(e,t){var n=t.ref;if(n===null)e!==null&&e.ref!==null&&(t.flags|=4194816);else{if(typeof n!=`function`&&typeof n!=`object`)throw Error(s(284));(e===null||e.ref!==n)&&(t.flags|=4194816)}}function xc(e,t,n,r,i){return oa(t),n=Ao(e,t,n,r,void 0,i),r=Po(),e!==null&&!lc?(Fo(e,t,i),Fc(e,t,i)):(M&&r&&Li(t),t.flags|=1,uc(e,t,n,i),t.child)}function Sc(e,t,n,r,i,a){return oa(t),t.updateQueue=null,n=Mo(t,r,n,i),jo(e),r=Po(),e!==null&&!lc?(Fo(e,t,a),Fc(e,t,a)):(M&&r&&Li(t),t.flags|=1,uc(e,t,n,a),t.child)}function Cc(e,t,n,r,i){if(oa(t),t.stateNode===null){var a=pi,o=n.contextType;typeof o==`object`&&o&&(a=sa(o)),a=new n(r,a),t.memoizedState=a.state!==null&&a.state!==void 0?a.state:null,a.updater=Ys,t.stateNode=a,a._reactInternals=t,a=t.stateNode,a.props=r,a.state=t.memoizedState,a.refs={},qa(t),o=n.contextType,a.context=typeof o==`object`&&o?sa(o):pi,a.state=t.memoizedState,o=n.getDerivedStateFromProps,typeof o==`function`&&(Js(t,n,o,r),a.state=t.memoizedState),typeof n.getDerivedStateFromProps==`function`||typeof a.getSnapshotBeforeUpdate==`function`||typeof a.UNSAFE_componentWillMount!=`function`&&typeof a.componentWillMount!=`function`||(o=a.state,typeof a.componentWillMount==`function`&&a.componentWillMount(),typeof a.UNSAFE_componentWillMount==`function`&&a.UNSAFE_componentWillMount(),o!==a.state&&Ys.enqueueReplaceState(a,a.state,null),to(t,r,a,i),eo(),a.state=t.memoizedState),typeof a.componentDidMount==`function`&&(t.flags|=4194308),r=!0}else if(e===null){a=t.stateNode;var s=t.memoizedProps,c=Qs(n,s);a.props=c;var l=a.context,u=n.contextType;o=pi,typeof u==`object`&&u&&(o=sa(u));var d=n.getDerivedStateFromProps;u=typeof d==`function`||typeof a.getSnapshotBeforeUpdate==`function`,s=t.pendingProps!==s,u||typeof a.UNSAFE_componentWillReceiveProps!=`function`&&typeof a.componentWillReceiveProps!=`function`||(s||l!==o)&&Zs(t,a,r,o),Ka=!1;var f=t.memoizedState;a.state=f,to(t,r,a,i),eo(),l=t.memoizedState,s||f!==l||Ka?(typeof d==`function`&&(Js(t,n,d,r),l=t.memoizedState),(c=Ka||Xs(t,n,c,r,f,l,o))?(u||typeof a.UNSAFE_componentWillMount!=`function`&&typeof a.componentWillMount!=`function`||(typeof a.componentWillMount==`function`&&a.componentWillMount(),typeof a.UNSAFE_componentWillMount==`function`&&a.UNSAFE_componentWillMount()),typeof a.componentDidMount==`function`&&(t.flags|=4194308)):(typeof a.componentDidMount==`function`&&(t.flags|=4194308),t.memoizedProps=r,t.memoizedState=l),a.props=r,a.state=l,a.context=o,r=c):(typeof a.componentDidMount==`function`&&(t.flags|=4194308),r=!1)}else{a=t.stateNode,Ja(e,t),o=t.memoizedProps,u=Qs(n,o),a.props=u,d=t.pendingProps,f=a.context,l=n.contextType,c=pi,typeof l==`object`&&l&&(c=sa(l)),s=n.getDerivedStateFromProps,(l=typeof s==`function`||typeof a.getSnapshotBeforeUpdate==`function`)||typeof a.UNSAFE_componentWillReceiveProps!=`function`&&typeof a.componentWillReceiveProps!=`function`||(o!==d||f!==c)&&Zs(t,a,r,c),Ka=!1,f=t.memoizedState,a.state=f,to(t,r,a,i),eo();var p=t.memoizedState;o!==d||f!==p||Ka||e!==null&&e.dependencies!==null&&aa(e.dependencies)?(typeof s==`function`&&(Js(t,n,s,r),p=t.memoizedState),(u=Ka||Xs(t,n,u,r,f,p,c)||e!==null&&e.dependencies!==null&&aa(e.dependencies))?(l||typeof a.UNSAFE_componentWillUpdate!=`function`&&typeof a.componentWillUpdate!=`function`||(typeof a.componentWillUpdate==`function`&&a.componentWillUpdate(r,p,c),typeof a.UNSAFE_componentWillUpdate==`function`&&a.UNSAFE_componentWillUpdate(r,p,c)),typeof a.componentDidUpdate==`function`&&(t.flags|=4),typeof a.getSnapshotBeforeUpdate==`function`&&(t.flags|=1024)):(typeof a.componentDidUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=4),typeof a.getSnapshotBeforeUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=1024),t.memoizedProps=r,t.memoizedState=p),a.props=r,a.state=p,a.context=c,r=u):(typeof a.componentDidUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=4),typeof a.getSnapshotBeforeUpdate!=`function`||o===e.memoizedProps&&f===e.memoizedState||(t.flags|=1024),r=!1)}return a=r,bc(e,t),r=(t.flags&128)!=0,a||r?(a=t.stateNode,n=r&&typeof n.getDerivedStateFromError!=`function`?null:a.render(),t.flags|=1,e!==null&&r?(t.child=Wa(t,e.child,null,i),t.child=Wa(t,null,n,i)):uc(e,t,n,i),t.memoizedState=a.state,e=t.child):e=Fc(e,t,i),e}function wc(e,t,n,r){return Yi(),t.flags|=256,uc(e,t,n,r),t.child}var Tc={dehydrated:null,treeContext:null,retryLane:0,hydrationErrors:null};function Ec(e){return{baseLanes:e,cachePool:Da()}}function Dc(e,t,n){return e=e===null?0:e.childLanes&~n,t&&(e|=eu),e}function Oc(e,t,n){var r=t.pendingProps,i=!1,a=(t.flags&128)!=0,o;if((o=a)||(o=e!==null&&e.memoizedState===null?!1:(go.current&2)!=0),o&&(i=!0,t.flags&=-129),o=(t.flags&32)!=0,t.flags&=-33,e===null){if(M){if(i?fo(t):mo(t),(e=Vi)?(e=sf(e,Ui),e=e!==null&&e.data!==`&`?e:null,e!==null&&(t.memoizedState={dehydrated:e,treeContext:Mi===null?null:{id:Ni,overflow:Pi},retryLane:536870912,hydrationErrors:null},n=Si(e),n.return=t,t.child=n,Bi=t,Vi=null)):e=null,e===null)throw Gi(t);return lf(e)?t.lanes=32:t.lanes=536870912,null}var c=r.children;return r=r.fallback,i?(mo(t),i=t.mode,c=Ac({mode:`hidden`,children:c},i),r=bi(r,i,n,null),c.return=t,r.return=t,c.sibling=r,t.child=c,r=t.child,r.memoizedState=Ec(n),r.childLanes=Dc(e,o,n),t.memoizedState=Tc,hc(null,r)):(fo(t),kc(t,c))}var l=e.memoizedState;if(l!==null&&(c=l.dehydrated,c!==null)){if(a)t.flags&256?(fo(t),t.flags&=-257,t=jc(e,t,n)):t.memoizedState===null?(mo(t),c=r.fallback,i=t.mode,r=Ac({mode:`visible`,children:r.children},i),c=bi(c,i,n,null),c.flags|=2,r.return=t,c.return=t,r.sibling=c,t.child=r,Wa(t,e.child,null,n),r=t.child,r.memoizedState=Ec(n),r.childLanes=Dc(e,o,n),t.memoizedState=Tc,t=hc(null,r)):(mo(t),t.child=e.child,t.flags|=128,t=null);else if(fo(t),lf(c)){if(o=c.nextSibling&&c.nextSibling.dataset,o)var u=o.dgst;o=u,r=Error(s(419)),r.stack=``,r.digest=o,Zi({value:r,source:null,stack:null}),t=jc(e,t,n)}else if(lc||ia(e,t,n,!1),o=(n&e.childLanes)!==0,lc||o){if(o=Gl,o!==null&&(r=ut(o,n),r!==0&&r!==l.retryLane))throw l.retryLane=r,ui(e,r),K(o,e,r),cc;cf(c)||ju(),t=jc(e,t,n)}else cf(c)?(t.flags|=192,t.child=e.child,t=null):(e=l.treeContext,Vi=df(c.nextSibling),Bi=t,M=!0,Hi=null,Ui=!1,e!==null&&zi(t,e),t=kc(t,r.children),t.flags|=4096);return t}return i?(mo(t),c=r.fallback,i=t.mode,l=e.child,u=l.sibling,r=_i(l,{mode:`hidden`,children:r.children}),r.subtreeFlags=l.subtreeFlags&65011712,u===null?(c=bi(c,i,n,null),c.flags|=2):c=_i(u,c),c.return=t,r.return=t,r.sibling=c,t.child=r,hc(null,r),r=t.child,c=e.child.memoizedState,c===null?c=Ec(n):(i=c.cachePool,i===null?i=Da():(l=pa._currentValue,i=i.parent===l?i:{parent:l,pool:l}),c={baseLanes:c.baseLanes|n,cachePool:i}),r.memoizedState=c,r.childLanes=Dc(e,o,n),t.memoizedState=Tc,hc(e.child,r)):(fo(t),n=e.child,e=n.sibling,n=_i(n,{mode:`visible`,children:r.children}),n.return=t,n.sibling=null,e!==null&&(o=t.deletions,o===null?(t.deletions=[e],t.flags|=16):o.push(e)),t.child=n,t.memoizedState=null,n)}function kc(e,t){return t=Ac({mode:`visible`,children:t},e.mode),t.return=e,e.child=t}function Ac(e,t){return e=hi(22,e,null,t),e.lanes=0,e}function jc(e,t,n){return Wa(t,e.child,null,n),e=kc(t,t.pendingProps.children),e.flags|=2,t.memoizedState=null,e}function Mc(e,t,n){e.lanes|=t;var r=e.alternate;r!==null&&(r.lanes|=t),ra(e.return,t,n)}function Nc(e,t,n,r,i,a){var o=e.memoizedState;o===null?e.memoizedState={isBackwards:t,rendering:null,renderingStartTime:0,last:r,tail:n,tailMode:i,treeForkCount:a}:(o.isBackwards=t,o.rendering=null,o.renderingStartTime=0,o.last=r,o.tail=n,o.tailMode=i,o.treeForkCount=a)}function Pc(e,t,n){var r=t.pendingProps,i=r.revealOrder,a=r.tail;r=r.children;var o=go.current,s=(o&2)!=0;if(s?(o=o&1|2,t.flags|=128):o&=1,me(go,o),uc(e,t,r,n),r=M?ki:0,!s&&e!==null&&e.flags&128)a:for(e=t.child;e!==null;){if(e.tag===13)e.memoizedState!==null&&Mc(e,n,t);else if(e.tag===19)Mc(e,n,t);else if(e.child!==null){e.child.return=e,e=e.child;continue}if(e===t)break a;for(;e.sibling===null;){if(e.return===null||e.return===t)break a;e=e.return}e.sibling.return=e.return,e=e.sibling}switch(i){case`forwards`:for(n=t.child,i=null;n!==null;)e=n.alternate,e!==null&&_o(e)===null&&(i=n),n=n.sibling;n=i,n===null?(i=t.child,t.child=null):(i=n.sibling,n.sibling=null),Nc(t,!1,i,n,a,r);break;case`backwards`:case`unstable_legacy-backwards`:for(n=null,i=t.child,t.child=null;i!==null;){if(e=i.alternate,e!==null&&_o(e)===null){t.child=i;break}e=i.sibling,i.sibling=n,n=i,i=e}Nc(t,!0,n,null,a,r);break;case`together`:Nc(t,!1,null,null,void 0,r);break;default:t.memoizedState=null}return t.child}function Fc(e,t,n){if(e!==null&&(t.dependencies=e.dependencies),Zl|=t.lanes,(n&t.childLanes)===0)if(e!==null){if(ia(e,t,n,!1),(n&t.childLanes)===0)return null}else return null;if(e!==null&&t.child!==e.child)throw Error(s(153));if(t.child!==null){for(e=t.child,n=_i(e,e.pendingProps),t.child=n,n.return=t;e.sibling!==null;)e=e.sibling,n=n.sibling=_i(e,e.pendingProps),n.return=t;n.sibling=null}return t.child}function Ic(e,t){return(e.lanes&t)===0?(e=e.dependencies,!!(e!==null&&aa(e))):!0}function Lc(e,t,n){switch(t.tag){case 3:ye(t,t.stateNode.containerInfo),ta(t,pa,e.memoizedState.cache),Yi();break;case 27:case 5:xe(t);break;case 4:ye(t,t.stateNode.containerInfo);break;case 10:ta(t,t.type,t.memoizedProps.value);break;case 31:if(t.memoizedState!==null)return t.flags|=128,P(t),null;break;case 13:var r=t.memoizedState;if(r!==null)return r.dehydrated===null?(n&t.child.childLanes)===0?(fo(t),e=Fc(e,t,n),e===null?null:e.sibling):Oc(e,t,n):(fo(t),t.flags|=128,null);fo(t);break;case 19:var i=(e.flags&128)!=0;if(r=(n&t.childLanes)!==0,r||=(ia(e,t,n,!1),(n&t.childLanes)!==0),i){if(r)return Pc(e,t,n);t.flags|=128}if(i=t.memoizedState,i!==null&&(i.rendering=null,i.tail=null,i.lastEffect=null),me(go,go.current),r)break;return null;case 22:return t.lanes=0,mc(e,t,n,t.pendingProps);case 24:ta(t,pa,e.memoizedState.cache)}return Fc(e,t,n)}function R(e,t,n){if(e!==null)if(e.memoizedProps!==t.pendingProps)lc=!0;else{if(!Ic(e,n)&&!(t.flags&128))return lc=!1,Lc(e,t,n);lc=!!(e.flags&131072)}else lc=!1,M&&t.flags&1048576&&Ii(t,ki,t.index);switch(t.lanes=0,t.tag){case 16:a:{var r=t.pendingProps;if(e=Pa(t.elementType),t.type=e,typeof e==`function`)gi(e)?(r=Qs(e,r),t.tag=1,t=Cc(null,t,e,r,n)):(t.tag=0,t=xc(null,t,e,r,n));else{if(e!=null){var i=e.$$typeof;if(i===w){t.tag=11,t=dc(null,t,e,r,n);break a}else if(i===T){t.tag=14,t=fc(null,t,e,r,n);break a}}throw t=se(e)||e,Error(s(306,t,``))}}return t;case 0:return xc(e,t,t.type,t.pendingProps,n);case 1:return r=t.type,i=Qs(r,t.pendingProps),Cc(e,t,r,i,n);case 3:a:{if(ye(t,t.stateNode.containerInfo),e===null)throw Error(s(387));r=t.pendingProps;var a=t.memoizedState;i=a.element,Ja(e,t),to(t,r,null,n);var o=t.memoizedState;if(r=o.cache,ta(t,pa,r),r!==a.cache&&N(t,[pa],n,!0),eo(),r=o.element,a.isDehydrated)if(a={element:r,isDehydrated:!1,cache:o.cache},t.updateQueue.baseState=a,t.memoizedState=a,t.flags&256){t=wc(e,t,r,n);break a}else if(r!==i){i=Ti(Error(s(424)),t),Zi(i),t=wc(e,t,r,n);break a}else{switch(e=t.stateNode.containerInfo,e.nodeType){case 9:e=e.body;break;default:e=e.nodeName===`HTML`?e.ownerDocument.body:e}for(Vi=df(e.firstChild),Bi=t,M=!0,Hi=null,Ui=!0,n=Ga(t,null,r,n),t.child=n;n;)n.flags=n.flags&-3|4096,n=n.sibling}else{if(Yi(),r===i){t=Fc(e,t,n);break a}uc(e,t,r,n)}t=t.child}return t;case 26:return bc(e,t),e===null?(n=Mf(t.type,null,t.pendingProps,null))?t.memoizedState=n:M||(n=t.type,e=t.pendingProps,r=Ud(_e.current).createElement(n),r[gt]=t,r[_t]=e,Rd(r,n,e),kt(r),t.stateNode=r):t.memoizedState=Mf(t.type,e.memoizedProps,t.pendingProps,e.memoizedState),null;case 27:return xe(t),e===null&&M&&(r=t.stateNode=hf(t.type,t.pendingProps,_e.current),Bi=t,Ui=!0,i=Vi,ef(t.type)?(ff=i,Vi=df(r.firstChild)):Vi=i),uc(e,t,t.pendingProps.children,n),bc(e,t),e===null&&(t.flags|=4194304),t.child;case 5:return e===null&&M&&((i=r=Vi)&&(r=af(r,t.type,t.pendingProps,Ui),r===null?i=!1:(t.stateNode=r,Bi=t,Vi=df(r.firstChild),Ui=!1,i=!0)),i||Gi(t)),xe(t),i=t.type,a=t.pendingProps,o=e===null?null:e.memoizedProps,r=a.children,Kd(i,a)?r=null:o!==null&&Kd(i,o)&&(t.flags|=32),t.memoizedState!==null&&(i=Ao(e,t,No,null,null,n),ep._currentValue=i),bc(e,t),uc(e,t,r,n),t.child;case 6:return e===null&&M&&((e=n=Vi)&&(n=of(n,t.pendingProps,Ui),n===null?e=!1:(t.stateNode=n,Bi=t,Vi=null,e=!0)),e||Gi(t)),null;case 13:return Oc(e,t,n);case 4:return ye(t,t.stateNode.containerInfo),r=t.pendingProps,e===null?t.child=Wa(t,null,r,n):uc(e,t,r,n),t.child;case 11:return dc(e,t,t.type,t.pendingProps,n);case 7:return uc(e,t,t.pendingProps,n),t.child;case 8:return uc(e,t,t.pendingProps.children,n),t.child;case 12:return uc(e,t,t.pendingProps.children,n),t.child;case 10:return r=t.pendingProps,ta(t,t.type,r.value),uc(e,t,r.children,n),t.child;case 9:return i=t.type._context,r=t.pendingProps.children,oa(t),i=sa(i),r=r(i),t.flags|=1,uc(e,t,r,n),t.child;case 14:return fc(e,t,t.type,t.pendingProps,n);case 15:return pc(e,t,t.type,t.pendingProps,n);case 19:return Pc(e,t,n);case 31:return yc(e,t,n);case 22:return mc(e,t,n,t.pendingProps);case 24:return oa(t),r=sa(pa),e===null?(i=Ta(),i===null&&(i=Gl,a=ma(),i.pooledCache=a,a.refCount++,a!==null&&(i.pooledCacheLanes|=n),i=a),t.memoizedState={parent:r,cache:i},qa(t),ta(t,pa,i)):((e.lanes&n)!==0&&(Ja(e,t),to(t,null,null,n),eo()),i=e.memoizedState,a=t.memoizedState,i.parent===r?(r=a.cache,ta(t,pa,r),r!==i.cache&&N(t,[pa],n,!0)):(i={parent:r,cache:r},t.memoizedState=i,t.lanes===0&&(t.memoizedState=t.updateQueue.baseState=i),ta(t,pa,r))),uc(e,t,t.pendingProps.children,n),t.child;case 29:throw t.pendingProps}throw Error(s(156,t.tag))}function Rc(e){e.flags|=4}function zc(e,t,n,r,i){if((t=(e.mode&32)!=0)&&(t=!1),t){if(e.flags|=16777216,(i&335544128)===i)if(e.stateNode.complete)e.flags|=8192;else if(Ou())e.flags|=8192;else throw Fa=ja,ka}else e.flags&=-16777217}function Bc(e,t){if(t.type!==`stylesheet`||t.state.loading&4)e.flags&=-16777217;else if(e.flags|=16777216,!Kf(t))if(Ou())e.flags|=8192;else throw Fa=ja,ka}function Vc(e,t){t!==null&&(e.flags|=4),e.flags&16384&&(t=e.tag===22?536870912:it(),e.lanes|=t,tu|=t)}function Hc(e,t){if(!M)switch(e.tailMode){case`hidden`:t=e.tail;for(var n=null;t!==null;)t.alternate!==null&&(n=t),t=t.sibling;n===null?e.tail=null:n.sibling=null;break;case`collapsed`:n=e.tail;for(var r=null;n!==null;)n.alternate!==null&&(r=n),n=n.sibling;r===null?t||e.tail===null?e.tail=null:e.tail.sibling=null:r.sibling=null}}function Uc(e){var t=e.alternate!==null&&e.alternate.child===e.child,n=0,r=0;if(t)for(var i=e.child;i!==null;)n|=i.lanes|i.childLanes,r|=i.subtreeFlags&65011712,r|=i.flags&65011712,i.return=e,i=i.sibling;else for(i=e.child;i!==null;)n|=i.lanes|i.childLanes,r|=i.subtreeFlags,r|=i.flags,i.return=e,i=i.sibling;return e.subtreeFlags|=r,e.childLanes=n,t}function Wc(e,t,n){var r=t.pendingProps;switch(Ri(t),t.tag){case 16:case 15:case 0:case 11:case 7:case 8:case 12:case 9:case 14:return Uc(t),null;case 1:return Uc(t),null;case 3:return n=t.stateNode,r=null,e!==null&&(r=e.memoizedState.cache),t.memoizedState.cache!==r&&(t.flags|=2048),na(pa),be(),n.pendingContext&&(n.context=n.pendingContext,n.pendingContext=null),(e===null||e.child===null)&&(Ji(t)?Rc(t):e===null||e.memoizedState.isDehydrated&&!(t.flags&256)||(t.flags|=1024,Xi())),Uc(t),null;case 26:var i=t.type,a=t.memoizedState;return e===null?(Rc(t),a===null?(Uc(t),zc(t,i,null,r,n)):(Uc(t),Bc(t,a))):a?a===e.memoizedState?(Uc(t),t.flags&=-16777217):(Rc(t),Uc(t),Bc(t,a)):(e=e.memoizedProps,e!==r&&Rc(t),Uc(t),zc(t,i,e,r,n)),null;case 27:if(Se(t),n=_e.current,i=t.type,e!==null&&t.stateNode!=null)e.memoizedProps!==r&&Rc(t);else{if(!r){if(t.stateNode===null)throw Error(s(166));return Uc(t),null}e=he.current,Ji(t)?Ki(t,e):(e=hf(i,r,n),t.stateNode=e,Rc(t))}return Uc(t),null;case 5:if(Se(t),i=t.type,e!==null&&t.stateNode!=null)e.memoizedProps!==r&&Rc(t);else{if(!r){if(t.stateNode===null)throw Error(s(166));return Uc(t),null}if(a=he.current,Ji(t))Ki(t,a);else{var o=Ud(_e.current);switch(a){case 1:a=o.createElementNS(`http://www.w3.org/2000/svg`,i);break;case 2:a=o.createElementNS(`http://www.w3.org/1998/Math/MathML`,i);break;default:switch(i){case`svg`:a=o.createElementNS(`http://www.w3.org/2000/svg`,i);break;case`math`:a=o.createElementNS(`http://www.w3.org/1998/Math/MathML`,i);break;case`script`:a=o.createElement(`div`),a.innerHTML=`<script><\/script>`,a=a.removeChild(a.firstChild);break;case`select`:a=typeof r.is==`string`?o.createElement(`select`,{is:r.is}):o.createElement(`select`),r.multiple?a.multiple=!0:r.size&&(a.size=r.size);break;default:a=typeof r.is==`string`?o.createElement(i,{is:r.is}):o.createElement(i)}}a[gt]=t,a[_t]=r;a:for(o=t.child;o!==null;){if(o.tag===5||o.tag===6)a.appendChild(o.stateNode);else if(o.tag!==4&&o.tag!==27&&o.child!==null){o.child.return=o,o=o.child;continue}if(o===t)break a;for(;o.sibling===null;){if(o.return===null||o.return===t)break a;o=o.return}o.sibling.return=o.return,o=o.sibling}t.stateNode=a;a:switch(Rd(a,i,r),i){case`button`:case`input`:case`select`:case`textarea`:r=!!r.autoFocus;break a;case`img`:r=!0;break a;default:r=!1}r&&Rc(t)}}return Uc(t),zc(t,t.type,e===null?null:e.memoizedProps,t.pendingProps,n),null;case 6:if(e&&t.stateNode!=null)e.memoizedProps!==r&&Rc(t);else{if(typeof r!=`string`&&t.stateNode===null)throw Error(s(166));if(e=_e.current,Ji(t)){if(e=t.stateNode,n=t.memoizedProps,r=null,i=Bi,i!==null)switch(i.tag){case 27:case 5:r=i.memoizedProps}e[gt]=t,e=!!(e.nodeValue===n||r!==null&&!0===r.suppressHydrationWarning||Fd(e.nodeValue,n)),e||Gi(t,!0)}else e=Ud(e).createTextNode(r),e[gt]=t,t.stateNode=e}return Uc(t),null;case 31:if(n=t.memoizedState,e===null||e.memoizedState!==null){if(r=Ji(t),n!==null){if(e===null){if(!r)throw Error(s(318));if(e=t.memoizedState,e=e===null?null:e.dehydrated,!e)throw Error(s(557));e[gt]=t}else Yi(),!(t.flags&128)&&(t.memoizedState=null),t.flags|=4;Uc(t),e=!1}else n=Xi(),e!==null&&e.memoizedState!==null&&(e.memoizedState.hydrationErrors=n),e=!0;if(!e)return t.flags&256?(ho(t),t):(ho(t),null);if(t.flags&128)throw Error(s(558))}return Uc(t),null;case 13:if(r=t.memoizedState,e===null||e.memoizedState!==null&&e.memoizedState.dehydrated!==null){if(i=Ji(t),r!==null&&r.dehydrated!==null){if(e===null){if(!i)throw Error(s(318));if(i=t.memoizedState,i=i===null?null:i.dehydrated,!i)throw Error(s(317));i[gt]=t}else Yi(),!(t.flags&128)&&(t.memoizedState=null),t.flags|=4;Uc(t),i=!1}else i=Xi(),e!==null&&e.memoizedState!==null&&(e.memoizedState.hydrationErrors=i),i=!0;if(!i)return t.flags&256?(ho(t),t):(ho(t),null)}return ho(t),t.flags&128?(t.lanes=n,t):(n=r!==null,e=e!==null&&e.memoizedState!==null,n&&(r=t.child,i=null,r.alternate!==null&&r.alternate.memoizedState!==null&&r.alternate.memoizedState.cachePool!==null&&(i=r.alternate.memoizedState.cachePool.pool),a=null,r.memoizedState!==null&&r.memoizedState.cachePool!==null&&(a=r.memoizedState.cachePool.pool),a!==i&&(r.flags|=2048)),n!==e&&n&&(t.child.flags|=8192),Vc(t,t.updateQueue),Uc(t),null);case 4:return be(),e===null&&Td(t.stateNode.containerInfo),Uc(t),null;case 10:return na(t.type),Uc(t),null;case 19:if(pe(go),r=t.memoizedState,r===null)return Uc(t),null;if(i=(t.flags&128)!=0,a=r.rendering,a===null)if(i)Hc(r,!1);else{if(W!==0||e!==null&&e.flags&128)for(e=t.child;e!==null;){if(a=_o(e),a!==null){for(t.flags|=128,Hc(r,!1),e=a.updateQueue,t.updateQueue=e,Vc(t,e),t.subtreeFlags=0,e=n,n=t.child;n!==null;)vi(n,e),n=n.sibling;return me(go,go.current&1|2),M&&Fi(t,r.treeForkCount),t.child}e=e.sibling}r.tail!==null&&Fe()>su&&(t.flags|=128,i=!0,Hc(r,!1),t.lanes=4194304)}else{if(!i)if(e=_o(a),e!==null){if(t.flags|=128,i=!0,e=e.updateQueue,t.updateQueue=e,Vc(t,e),Hc(r,!0),r.tail===null&&r.tailMode===`hidden`&&!a.alternate&&!M)return Uc(t),null}else 2*Fe()-r.renderingStartTime>su&&n!==536870912&&(t.flags|=128,i=!0,Hc(r,!1),t.lanes=4194304);r.isBackwards?(a.sibling=t.child,t.child=a):(e=r.last,e===null?t.child=a:e.sibling=a,r.last=a)}return r.tail===null?(Uc(t),null):(e=r.tail,r.rendering=e,r.tail=e.sibling,r.renderingStartTime=Fe(),e.sibling=null,n=go.current,me(go,i?n&1|2:n&1),M&&Fi(t,r.treeForkCount),e);case 22:case 23:return ho(t),co(),r=t.memoizedState!==null,e===null?r&&(t.flags|=8192):e.memoizedState!==null!==r&&(t.flags|=8192),r?n&536870912&&!(t.flags&128)&&(Uc(t),t.subtreeFlags&6&&(t.flags|=8192)):Uc(t),n=t.updateQueue,n!==null&&Vc(t,n.retryQueue),n=null,e!==null&&e.memoizedState!==null&&e.memoizedState.cachePool!==null&&(n=e.memoizedState.cachePool.pool),r=null,t.memoizedState!==null&&t.memoizedState.cachePool!==null&&(r=t.memoizedState.cachePool.pool),r!==n&&(t.flags|=2048),e!==null&&pe(wa),null;case 24:return n=null,e!==null&&(n=e.memoizedState.cache),t.memoizedState.cache!==n&&(t.flags|=2048),na(pa),Uc(t),null;case 25:return null;case 30:return null}throw Error(s(156,t.tag))}function Gc(e,t){switch(Ri(t),t.tag){case 1:return e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 3:return na(pa),be(),e=t.flags,e&65536&&!(e&128)?(t.flags=e&-65537|128,t):null;case 26:case 27:case 5:return Se(t),null;case 31:if(t.memoizedState!==null){if(ho(t),t.alternate===null)throw Error(s(340));Yi()}return e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 13:if(ho(t),e=t.memoizedState,e!==null&&e.dehydrated!==null){if(t.alternate===null)throw Error(s(340));Yi()}return e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 19:return pe(go),null;case 4:return be(),null;case 10:return na(t.type),null;case 22:case 23:return ho(t),co(),e!==null&&pe(wa),e=t.flags,e&65536?(t.flags=e&-65537|128,t):null;case 24:return na(pa),null;case 25:return null;default:return null}}function Kc(e,t){switch(Ri(t),t.tag){case 3:na(pa),be();break;case 26:case 27:case 5:Se(t);break;case 4:be();break;case 31:t.memoizedState!==null&&ho(t);break;case 13:ho(t);break;case 19:pe(go);break;case 10:na(t.type);break;case 22:case 23:ho(t),co(),e!==null&&pe(wa);break;case 24:na(pa)}}function qc(e,t){try{var n=t.updateQueue,r=n===null?null:n.lastEffect;if(r!==null){var i=r.next;n=i;do{if((n.tag&e)===e){r=void 0;var a=n.create,o=n.inst;r=a(),o.destroy=r}n=n.next}while(n!==i)}}catch(e){J(t,t.return,e)}}function Jc(e,t,n){try{var r=t.updateQueue,i=r===null?null:r.lastEffect;if(i!==null){var a=i.next;r=a;do{if((r.tag&e)===e){var o=r.inst,s=o.destroy;if(s!==void 0){o.destroy=void 0,i=t;var c=n,l=s;try{l()}catch(e){J(i,c,e)}}}r=r.next}while(r!==a)}}catch(e){J(t,t.return,e)}}function Yc(e){var t=e.updateQueue;if(t!==null){var n=e.stateNode;try{ro(t,n)}catch(t){J(e,e.return,t)}}}function Xc(e,t,n){n.props=Qs(e.type,e.memoizedProps),n.state=e.memoizedState;try{n.componentWillUnmount()}catch(n){J(e,t,n)}}function Zc(e,t){try{var n=e.ref;if(n!==null){switch(e.tag){case 26:case 27:case 5:var r=e.stateNode;break;case 30:r=e.stateNode;break;default:r=e.stateNode}typeof n==`function`?e.refCleanup=n(r):n.current=r}}catch(n){J(e,t,n)}}function Qc(e,t){var n=e.ref,r=e.refCleanup;if(n!==null)if(typeof r==`function`)try{r()}catch(n){J(e,t,n)}finally{e.refCleanup=null,e=e.alternate,e!=null&&(e.refCleanup=null)}else if(typeof n==`function`)try{n(null)}catch(n){J(e,t,n)}else n.current=null}function $c(e){var t=e.type,n=e.memoizedProps,r=e.stateNode;try{a:switch(t){case`button`:case`input`:case`select`:case`textarea`:n.autoFocus&&r.focus();break a;case`img`:n.src?r.src=n.src:n.srcSet&&(r.srcset=n.srcSet)}}catch(t){J(e,e.return,t)}}function el(e,t,n){try{var r=e.stateNode;zd(r,e.type,n,t),r[_t]=t}catch(t){J(e,e.return,t)}}function tl(e){return e.tag===5||e.tag===3||e.tag===26||e.tag===27&&ef(e.type)||e.tag===4}function nl(e){a:for(;;){for(;e.sibling===null;){if(e.return===null||tl(e.return))return null;e=e.return}for(e.sibling.return=e.return,e=e.sibling;e.tag!==5&&e.tag!==6&&e.tag!==18;){if(e.tag===27&&ef(e.type)||e.flags&2||e.child===null||e.tag===4)continue a;e.child.return=e,e=e.child}if(!(e.flags&2))return e.stateNode}}function rl(e,t,n){var r=e.tag;if(r===5||r===6)e=e.stateNode,t?(n.nodeType===9?n.body:n.nodeName===`HTML`?n.ownerDocument.body:n).insertBefore(e,t):(t=n.nodeType===9?n.body:n.nodeName===`HTML`?n.ownerDocument.body:n,t.appendChild(e),n=n._reactRootContainer,n!=null||t.onclick!==null||(t.onclick=ln));else if(r!==4&&(r===27&&ef(e.type)&&(n=e.stateNode,t=null),e=e.child,e!==null))for(rl(e,t,n),e=e.sibling;e!==null;)rl(e,t,n),e=e.sibling}function il(e,t,n){var r=e.tag;if(r===5||r===6)e=e.stateNode,t?n.insertBefore(e,t):n.appendChild(e);else if(r!==4&&(r===27&&ef(e.type)&&(n=e.stateNode),e=e.child,e!==null))for(il(e,t,n),e=e.sibling;e!==null;)il(e,t,n),e=e.sibling}function al(e){var t=e.stateNode,n=e.memoizedProps;try{for(var r=e.type,i=t.attributes;i.length;)t.removeAttributeNode(i[0]);Rd(t,r,n),t[gt]=e,t[_t]=n}catch(t){J(e,e.return,t)}}var ol=!1,sl=!1,cl=!1,ll=typeof WeakSet==`function`?WeakSet:Set,ul=null;function dl(e,t){if(e=e.containerInfo,X=lp,e=Fr(e),Ir(e)){if(`selectionStart`in e)var n={start:e.selectionStart,end:e.selectionEnd};else a:{n=(n=e.ownerDocument)&&n.defaultView||window;var r=n.getSelection&&n.getSelection();if(r&&r.rangeCount!==0){n=r.anchorNode;var i=r.anchorOffset,a=r.focusNode;r=r.focusOffset;try{n.nodeType,a.nodeType}catch{n=null;break a}var o=0,c=-1,l=-1,u=0,d=0,f=e,p=null;b:for(;;){for(var m;f!==n||i!==0&&f.nodeType!==3||(c=o+i),f!==a||r!==0&&f.nodeType!==3||(l=o+r),f.nodeType===3&&(o+=f.nodeValue.length),(m=f.firstChild)!==null;)p=f,f=m;for(;;){if(f===e)break b;if(p===n&&++u===i&&(c=o),p===a&&++d===r&&(l=o),(m=f.nextSibling)!==null)break;f=p,p=f.parentNode}f=m}n=c===-1||l===-1?null:{start:c,end:l}}else n=null}n||={start:0,end:0}}else n=null;for(Hd={focusedElem:e,selectionRange:n},lp=!1,ul=t;ul!==null;)if(t=ul,e=t.child,t.subtreeFlags&1028&&e!==null)e.return=t,ul=e;else for(;ul!==null;){switch(t=ul,a=t.alternate,e=t.flags,t.tag){case 0:if(e&4&&(e=t.updateQueue,e=e===null?null:e.events,e!==null))for(n=0;n<e.length;n++)i=e[n],i.ref.impl=i.nextImpl;break;case 11:case 15:break;case 1:if(e&1024&&a!==null){e=void 0,n=t,i=a.memoizedProps,a=a.memoizedState,r=n.stateNode;try{var h=Qs(n.type,i);e=r.getSnapshotBeforeUpdate(h,a),r.__reactInternalSnapshotBeforeUpdate=e}catch(e){J(n,n.return,e)}}break;case 3:if(e&1024){if(e=t.stateNode.containerInfo,n=e.nodeType,n===9)rf(e);else if(n===1)switch(e.nodeName){case`HEAD`:case`HTML`:case`BODY`:rf(e);break;default:e.textContent=``}}break;case 5:case 26:case 27:case 6:case 4:case 17:break;default:if(e&1024)throw Error(s(163))}if(e=t.sibling,e!==null){e.return=t.return,ul=e;break}ul=t.return}}function fl(e,t,n){var r=n.flags;switch(n.tag){case 0:case 11:case 15:z(e,n),r&4&&qc(5,n);break;case 1:if(z(e,n),r&4)if(e=n.stateNode,t===null)try{e.componentDidMount()}catch(e){J(n,n.return,e)}else{var i=Qs(n.type,t.memoizedProps);t=t.memoizedState;try{e.componentDidUpdate(i,t,e.__reactInternalSnapshotBeforeUpdate)}catch(e){J(n,n.return,e)}}r&64&&Yc(n),r&512&&Zc(n,n.return);break;case 3:if(z(e,n),r&64&&(e=n.updateQueue,e!==null)){if(t=null,n.child!==null)switch(n.child.tag){case 27:case 5:t=n.child.stateNode;break;case 1:t=n.child.stateNode}try{ro(e,t)}catch(e){J(n,n.return,e)}}break;case 27:t===null&&r&4&&al(n);case 26:case 5:z(e,n),t===null&&r&4&&$c(n),r&512&&Zc(n,n.return);break;case 12:z(e,n);break;case 31:z(e,n),r&4&&vl(e,n);break;case 13:z(e,n),r&4&&yl(e,n),r&64&&(e=n.memoizedState,e!==null&&(e=e.dehydrated,e!==null&&(n=Zu.bind(null,n),uf(e,n))));break;case 22:if(r=n.memoizedState!==null||ol,!r){t=t!==null&&t.memoizedState!==null||sl,i=ol;var a=sl;ol=r,(sl=t)&&!a?Ol(e,n,(n.subtreeFlags&8772)!=0):z(e,n),ol=i,sl=a}break;case 30:break;default:z(e,n)}}function pl(e){var t=e.alternate;t!==null&&(e.alternate=null,pl(t)),e.child=null,e.deletions=null,e.sibling=null,e.tag===5&&(t=e.stateNode,t!==null&&wt(t)),e.stateNode=null,e.return=null,e.dependencies=null,e.memoizedProps=null,e.memoizedState=null,e.pendingProps=null,e.stateNode=null,e.updateQueue=null}var ml=null,hl=!1;function gl(e,t,n){for(n=n.child;n!==null;)_l(e,t,n),n=n.sibling}function _l(e,t,n){if(Ge&&typeof Ge.onCommitFiberUnmount==`function`)try{Ge.onCommitFiberUnmount(We,n)}catch{}switch(n.tag){case 26:sl||Qc(n,t),gl(e,t,n),n.memoizedState?n.memoizedState.count--:n.stateNode&&(n=n.stateNode,n.parentNode.removeChild(n));break;case 27:sl||Qc(n,t);var r=ml,i=hl;ef(n.type)&&(ml=n.stateNode,hl=!1),gl(e,t,n),gf(n.stateNode),ml=r,hl=i;break;case 5:sl||Qc(n,t);case 6:if(r=ml,i=hl,ml=null,gl(e,t,n),ml=r,hl=i,ml!==null)if(hl)try{(ml.nodeType===9?ml.body:ml.nodeName===`HTML`?ml.ownerDocument.body:ml).removeChild(n.stateNode)}catch(e){J(n,t,e)}else try{ml.removeChild(n.stateNode)}catch(e){J(n,t,e)}break;case 18:ml!==null&&(hl?(e=ml,tf(e.nodeType===9?e.body:e.nodeName===`HTML`?e.ownerDocument.body:e,n.stateNode),Fp(e)):tf(ml,n.stateNode));break;case 4:r=ml,i=hl,ml=n.stateNode.containerInfo,hl=!0,gl(e,t,n),ml=r,hl=i;break;case 0:case 11:case 14:case 15:Jc(2,n,t),sl||Jc(4,n,t),gl(e,t,n);break;case 1:sl||(Qc(n,t),r=n.stateNode,typeof r.componentWillUnmount==`function`&&Xc(n,t,r)),gl(e,t,n);break;case 21:gl(e,t,n);break;case 22:sl=(r=sl)||n.memoizedState!==null,gl(e,t,n),sl=r;break;default:gl(e,t,n)}}function vl(e,t){if(t.memoizedState===null&&(e=t.alternate,e!==null&&(e=e.memoizedState,e!==null))){e=e.dehydrated;try{Fp(e)}catch(e){J(t,t.return,e)}}}function yl(e,t){if(t.memoizedState===null&&(e=t.alternate,e!==null&&(e=e.memoizedState,e!==null&&(e=e.dehydrated,e!==null))))try{Fp(e)}catch(e){J(t,t.return,e)}}function bl(e){switch(e.tag){case 31:case 13:case 19:var t=e.stateNode;return t===null&&(t=e.stateNode=new ll),t;case 22:return e=e.stateNode,t=e._retryCache,t===null&&(t=e._retryCache=new ll),t;default:throw Error(s(435,e.tag))}}function xl(e,t){var n=bl(e);t.forEach(function(t){if(!n.has(t)){n.add(t);var r=Qu.bind(null,e,t);t.then(r,r)}})}function Sl(e,t){var n=t.deletions;if(n!==null)for(var r=0;r<n.length;r++){var i=n[r],a=e,o=t,c=o;a:for(;c!==null;){switch(c.tag){case 27:if(ef(c.type)){ml=c.stateNode,hl=!1;break a}break;case 5:ml=c.stateNode,hl=!1;break a;case 3:case 4:ml=c.stateNode.containerInfo,hl=!0;break a}c=c.return}if(ml===null)throw Error(s(160));_l(a,o,i),ml=null,hl=!1,a=i.alternate,a!==null&&(a.return=null),i.return=null}if(t.subtreeFlags&13886)for(t=t.child;t!==null;)wl(t,e),t=t.sibling}var Cl=null;function wl(e,t){var n=e.alternate,r=e.flags;switch(e.tag){case 0:case 11:case 14:case 15:Sl(t,e),Tl(e),r&4&&(Jc(3,e,e.return),qc(3,e),Jc(5,e,e.return));break;case 1:Sl(t,e),Tl(e),r&512&&(sl||n===null||Qc(n,n.return)),r&64&&ol&&(e=e.updateQueue,e!==null&&(r=e.callbacks,r!==null&&(n=e.shared.hiddenCallbacks,e.shared.hiddenCallbacks=n===null?r:n.concat(r))));break;case 26:var i=Cl;if(Sl(t,e),Tl(e),r&512&&(sl||n===null||Qc(n,n.return)),r&4){var a=n===null?null:n.memoizedState;if(r=e.memoizedState,n===null)if(r===null)if(e.stateNode===null){a:{r=e.type,n=e.memoizedProps,i=i.ownerDocument||i;b:switch(r){case`title`:a=i.getElementsByTagName(`title`)[0],(!a||a[Ct]||a[gt]||a.namespaceURI===`http://www.w3.org/2000/svg`||a.hasAttribute(`itemprop`))&&(a=i.createElement(r),i.head.insertBefore(a,i.querySelector(`head > title`))),Rd(a,r,n),a[gt]=e,kt(a),r=a;break a;case`link`:var o=Uf(`link`,`href`,i).get(r+(n.href||``));if(o){for(var c=0;c<o.length;c++)if(a=o[c],a.getAttribute(`href`)===(n.href==null||n.href===``?null:n.href)&&a.getAttribute(`rel`)===(n.rel==null?null:n.rel)&&a.getAttribute(`title`)===(n.title==null?null:n.title)&&a.getAttribute(`crossorigin`)===(n.crossOrigin==null?null:n.crossOrigin)){o.splice(c,1);break b}}a=i.createElement(r),Rd(a,r,n),i.head.appendChild(a);break;case`meta`:if(o=Uf(`meta`,`content`,i).get(r+(n.content||``))){for(c=0;c<o.length;c++)if(a=o[c],a.getAttribute(`content`)===(n.content==null?null:``+n.content)&&a.getAttribute(`name`)===(n.name==null?null:n.name)&&a.getAttribute(`property`)===(n.property==null?null:n.property)&&a.getAttribute(`http-equiv`)===(n.httpEquiv==null?null:n.httpEquiv)&&a.getAttribute(`charset`)===(n.charSet==null?null:n.charSet)){o.splice(c,1);break b}}a=i.createElement(r),Rd(a,r,n),i.head.appendChild(a);break;default:throw Error(s(468,r))}a[gt]=e,kt(a),r=a}e.stateNode=r}else Wf(i,e.type,e.stateNode);else e.stateNode=Rf(i,r,e.memoizedProps);else a===r?r===null&&e.stateNode!==null&&el(e,e.memoizedProps,n.memoizedProps):(a===null?n.stateNode!==null&&(n=n.stateNode,n.parentNode.removeChild(n)):a.count--,r===null?Wf(i,e.type,e.stateNode):Rf(i,r,e.memoizedProps))}break;case 27:Sl(t,e),Tl(e),r&512&&(sl||n===null||Qc(n,n.return)),n!==null&&r&4&&el(e,e.memoizedProps,n.memoizedProps);break;case 5:if(Sl(t,e),Tl(e),r&512&&(sl||n===null||Qc(n,n.return)),e.flags&32){i=e.stateNode;try{k(i,``)}catch(t){J(e,e.return,t)}}r&4&&e.stateNode!=null&&(i=e.memoizedProps,el(e,i,n===null?i:n.memoizedProps)),r&1024&&(cl=!0);break;case 6:if(Sl(t,e),Tl(e),r&4){if(e.stateNode===null)throw Error(s(162));r=e.memoizedProps,n=e.stateNode;try{n.nodeValue=r}catch(t){J(e,e.return,t)}}break;case 3:if(Hf=null,i=Cl,Cl=yf(t.containerInfo),Sl(t,e),Cl=i,Tl(e),r&4&&n!==null&&n.memoizedState.isDehydrated)try{Fp(t.containerInfo)}catch(t){J(e,e.return,t)}cl&&(cl=!1,El(e));break;case 4:r=Cl,Cl=yf(e.stateNode.containerInfo),Sl(t,e),Tl(e),Cl=r;break;case 12:Sl(t,e),Tl(e);break;case 31:Sl(t,e),Tl(e),r&4&&(r=e.updateQueue,r!==null&&(e.updateQueue=null,xl(e,r)));break;case 13:Sl(t,e),Tl(e),e.child.flags&8192&&e.memoizedState!==null!=(n!==null&&n.memoizedState!==null)&&(au=Fe()),r&4&&(r=e.updateQueue,r!==null&&(e.updateQueue=null,xl(e,r)));break;case 22:i=e.memoizedState!==null;var l=n!==null&&n.memoizedState!==null,u=ol,d=sl;if(ol=u||i,sl=d||l,Sl(t,e),sl=d,ol=u,Tl(e),r&8192)a:for(t=e.stateNode,t._visibility=i?t._visibility&-2:t._visibility|1,i&&(n===null||l||ol||sl||Dl(e)),n=null,t=e;;){if(t.tag===5||t.tag===26){if(n===null){l=n=t;try{if(a=l.stateNode,i)o=a.style,typeof o.setProperty==`function`?o.setProperty(`display`,`none`,`important`):o.display=`none`;else{c=l.stateNode;var f=l.memoizedProps.style,p=f!=null&&f.hasOwnProperty(`display`)?f.display:null;c.style.display=p==null||typeof p==`boolean`?``:(``+p).trim()}}catch(e){J(l,l.return,e)}}}else if(t.tag===6){if(n===null){l=t;try{l.stateNode.nodeValue=i?``:l.memoizedProps}catch(e){J(l,l.return,e)}}}else if(t.tag===18){if(n===null){l=t;try{var m=l.stateNode;i?nf(m,!0):nf(l.stateNode,!1)}catch(e){J(l,l.return,e)}}}else if((t.tag!==22&&t.tag!==23||t.memoizedState===null||t===e)&&t.child!==null){t.child.return=t,t=t.child;continue}if(t===e)break a;for(;t.sibling===null;){if(t.return===null||t.return===e)break a;n===t&&(n=null),t=t.return}n===t&&(n=null),t.sibling.return=t.return,t=t.sibling}r&4&&(r=e.updateQueue,r!==null&&(n=r.retryQueue,n!==null&&(r.retryQueue=null,xl(e,n))));break;case 19:Sl(t,e),Tl(e),r&4&&(r=e.updateQueue,r!==null&&(e.updateQueue=null,xl(e,r)));break;case 30:break;case 21:break;default:Sl(t,e),Tl(e)}}function Tl(e){var t=e.flags;if(t&2){try{for(var n,r=e.return;r!==null;){if(tl(r)){n=r;break}r=r.return}if(n==null)throw Error(s(160));switch(n.tag){case 27:var i=n.stateNode;il(e,nl(e),i);break;case 5:var a=n.stateNode;n.flags&32&&(k(a,``),n.flags&=-33),il(e,nl(e),a);break;case 3:case 4:var o=n.stateNode.containerInfo;rl(e,nl(e),o);break;default:throw Error(s(161))}}catch(t){J(e,e.return,t)}e.flags&=-3}t&4096&&(e.flags&=-4097)}function El(e){if(e.subtreeFlags&1024)for(e=e.child;e!==null;){var t=e;El(t),t.tag===5&&t.flags&1024&&t.stateNode.reset(),e=e.sibling}}function z(e,t){if(t.subtreeFlags&8772)for(t=t.child;t!==null;)fl(e,t.alternate,t),t=t.sibling}function Dl(e){for(e=e.child;e!==null;){var t=e;switch(t.tag){case 0:case 11:case 14:case 15:Jc(4,t,t.return),Dl(t);break;case 1:Qc(t,t.return);var n=t.stateNode;typeof n.componentWillUnmount==`function`&&Xc(t,t.return,n),Dl(t);break;case 27:gf(t.stateNode);case 26:case 5:Qc(t,t.return),Dl(t);break;case 22:t.memoizedState===null&&Dl(t);break;case 30:Dl(t);break;default:Dl(t)}e=e.sibling}}function Ol(e,t,n){for(n&&=(t.subtreeFlags&8772)!=0,t=t.child;t!==null;){var r=t.alternate,i=e,a=t,o=a.flags;switch(a.tag){case 0:case 11:case 15:Ol(i,a,n),qc(4,a);break;case 1:if(Ol(i,a,n),r=a,i=r.stateNode,typeof i.componentDidMount==`function`)try{i.componentDidMount()}catch(e){J(r,r.return,e)}if(r=a,i=r.updateQueue,i!==null){var s=r.stateNode;try{var c=i.shared.hiddenCallbacks;if(c!==null)for(i.shared.hiddenCallbacks=null,i=0;i<c.length;i++)no(c[i],s)}catch(e){J(r,r.return,e)}}n&&o&64&&Yc(a),Zc(a,a.return);break;case 27:al(a);case 26:case 5:Ol(i,a,n),n&&r===null&&o&4&&$c(a),Zc(a,a.return);break;case 12:Ol(i,a,n);break;case 31:Ol(i,a,n),n&&o&4&&vl(i,a);break;case 13:Ol(i,a,n),n&&o&4&&yl(i,a);break;case 22:a.memoizedState===null&&Ol(i,a,n),Zc(a,a.return);break;case 30:break;default:Ol(i,a,n)}t=t.sibling}}function kl(e,t){var n=null;e!==null&&e.memoizedState!==null&&e.memoizedState.cachePool!==null&&(n=e.memoizedState.cachePool.pool),e=null,t.memoizedState!==null&&t.memoizedState.cachePool!==null&&(e=t.memoizedState.cachePool.pool),e!==n&&(e!=null&&e.refCount++,n!=null&&ha(n))}function Al(e,t){e=null,t.alternate!==null&&(e=t.alternate.memoizedState.cache),t=t.memoizedState.cache,t!==e&&(t.refCount++,e!=null&&ha(e))}function jl(e,t,n,r){if(t.subtreeFlags&10256)for(t=t.child;t!==null;)Ml(e,t,n,r),t=t.sibling}function Ml(e,t,n,r){var i=t.flags;switch(t.tag){case 0:case 11:case 15:jl(e,t,n,r),i&2048&&qc(9,t);break;case 1:jl(e,t,n,r);break;case 3:jl(e,t,n,r),i&2048&&(e=null,t.alternate!==null&&(e=t.alternate.memoizedState.cache),t=t.memoizedState.cache,t!==e&&(t.refCount++,e!=null&&ha(e)));break;case 12:if(i&2048){jl(e,t,n,r),e=t.stateNode;try{var a=t.memoizedProps,o=a.id,s=a.onPostCommit;typeof s==`function`&&s(o,t.alternate===null?`mount`:`update`,e.passiveEffectDuration,-0)}catch(e){J(t,t.return,e)}}else jl(e,t,n,r);break;case 31:jl(e,t,n,r);break;case 13:jl(e,t,n,r);break;case 23:break;case 22:a=t.stateNode,o=t.alternate,t.memoizedState===null?a._visibility&2?jl(e,t,n,r):(a._visibility|=2,Nl(e,t,n,r,(t.subtreeFlags&10256)!=0||!1)):a._visibility&2?jl(e,t,n,r):Pl(e,t),i&2048&&kl(o,t);break;case 24:jl(e,t,n,r),i&2048&&Al(t.alternate,t);break;default:jl(e,t,n,r)}}function Nl(e,t,n,r,i){for(i&&=(t.subtreeFlags&10256)!=0||!1,t=t.child;t!==null;){var a=e,o=t,s=n,c=r,l=o.flags;switch(o.tag){case 0:case 11:case 15:Nl(a,o,s,c,i),qc(8,o);break;case 23:break;case 22:var u=o.stateNode;o.memoizedState===null?(u._visibility|=2,Nl(a,o,s,c,i)):u._visibility&2?Nl(a,o,s,c,i):Pl(a,o),i&&l&2048&&kl(o.alternate,o);break;case 24:Nl(a,o,s,c,i),i&&l&2048&&Al(o.alternate,o);break;default:Nl(a,o,s,c,i)}t=t.sibling}}function Pl(e,t){if(t.subtreeFlags&10256)for(t=t.child;t!==null;){var n=e,r=t,i=r.flags;switch(r.tag){case 22:Pl(n,r),i&2048&&kl(r.alternate,r);break;case 24:Pl(n,r),i&2048&&Al(r.alternate,r);break;default:Pl(n,r)}t=t.sibling}}var Fl=8192;function Il(e,t,n){if(e.subtreeFlags&Fl)for(e=e.child;e!==null;)Ll(e,t,n),e=e.sibling}function Ll(e,t,n){switch(e.tag){case 26:Il(e,t,n),e.flags&Fl&&e.memoizedState!==null&&qf(n,Cl,e.memoizedState,e.memoizedProps);break;case 5:Il(e,t,n);break;case 3:case 4:var r=Cl;Cl=yf(e.stateNode.containerInfo),Il(e,t,n),Cl=r;break;case 22:e.memoizedState===null&&(r=e.alternate,r!==null&&r.memoizedState!==null?(r=Fl,Fl=16777216,Il(e,t,n),Fl=r):Il(e,t,n));break;default:Il(e,t,n)}}function Rl(e){var t=e.alternate;if(t!==null&&(e=t.child,e!==null)){t.child=null;do t=e.sibling,e.sibling=null,e=t;while(e!==null)}}function zl(e){var t=e.deletions;if(e.flags&16){if(t!==null)for(var n=0;n<t.length;n++){var r=t[n];ul=r,Hl(r,e)}Rl(e)}if(e.subtreeFlags&10256)for(e=e.child;e!==null;)Bl(e),e=e.sibling}function Bl(e){switch(e.tag){case 0:case 11:case 15:zl(e),e.flags&2048&&Jc(9,e,e.return);break;case 3:zl(e);break;case 12:zl(e);break;case 22:var t=e.stateNode;e.memoizedState!==null&&t._visibility&2&&(e.return===null||e.return.tag!==13)?(t._visibility&=-3,Vl(e)):zl(e);break;default:zl(e)}}function Vl(e){var t=e.deletions;if(e.flags&16){if(t!==null)for(var n=0;n<t.length;n++){var r=t[n];ul=r,Hl(r,e)}Rl(e)}for(e=e.child;e!==null;){switch(t=e,t.tag){case 0:case 11:case 15:Jc(8,t,t.return),Vl(t);break;case 22:n=t.stateNode,n._visibility&2&&(n._visibility&=-3,Vl(t));break;default:Vl(t)}e=e.sibling}}function Hl(e,t){for(;ul!==null;){var n=ul;switch(n.tag){case 0:case 11:case 15:Jc(8,n,t);break;case 23:case 22:if(n.memoizedState!==null&&n.memoizedState.cachePool!==null){var r=n.memoizedState.cachePool.pool;r!=null&&r.refCount++}break;case 24:ha(n.memoizedState.cache)}if(r=n.child,r!==null)r.return=n,ul=r;else a:for(n=e;ul!==null;){r=ul;var i=r.sibling,a=r.return;if(pl(r),r===n){ul=null;break a}if(i!==null){i.return=a,ul=i;break a}ul=a}}}var Ul={getCacheForType:function(e){var t=sa(pa),n=t.data.get(e);return n===void 0&&(n=e(),t.data.set(e,n)),n},cacheSignal:function(){return sa(pa).controller.signal}},Wl=typeof WeakMap==`function`?WeakMap:Map,B=0,Gl=null,V=null,H=0,Kl=0,ql=null,Jl=!1,Yl=!1,Xl=!1,U=0,W=0,Zl=0,Ql=0,$l=0,eu=0,tu=0,nu=null,ru=null,iu=!1,au=0,ou=0,su=1/0,cu=null,G=null,lu=0,uu=null,du=null,fu=0,pu=0,mu=null,hu=null,gu=0,_u=null;function vu(){return B&2&&H!==0?H&-H:D.T===null?pt():md()}function yu(){if(eu===0)if(!(H&536870912)||M){var e=Qe;Qe<<=1,!(Qe&3932160)&&(Qe=262144),eu=e}else eu=536870912;return e=lo.current,e!==null&&(e.flags|=32),eu}function K(e,t,n){(e===Gl&&(Kl===2||Kl===9)||e.cancelPendingCommit!==null)&&(Eu(e,0),Cu(e,H,eu,!1)),ot(e,n),(!(B&2)||e!==Gl)&&(e===Gl&&(!(B&2)&&(Ql|=n),W===4&&Cu(e,H,eu,!1)),od(e))}function bu(e,t,n){if(B&6)throw Error(s(327));var r=!n&&(t&127)==0&&(t&e.expiredLanes)===0||nt(e,t),i=r?Pu(e,t):Mu(e,t,!0),a=r;do{if(i===0){Yl&&!r&&Cu(e,t,0,!1);break}else{if(n=e.current.alternate,a&&!Su(n)){i=Mu(e,t,!1),a=!1;continue}if(i===2){if(a=t,e.errorRecoveryDisabledLanes&a)var o=0;else o=e.pendingLanes&-536870913,o=o===0?o&536870912?536870912:0:o;if(o!==0){t=o;a:{var c=e;i=nu;var l=c.current.memoizedState.isDehydrated;if(l&&(Eu(c,o).flags|=256),o=Mu(c,o,!1),o!==2){if(Xl&&!l){c.errorRecoveryDisabledLanes|=a,Ql|=a,i=4;break a}a=ru,ru=i,a!==null&&(ru===null?ru=a:ru.push.apply(ru,a))}i=o}if(a=!1,i!==2)continue}}if(i===1){Eu(e,0),Cu(e,t,0,!0);break}a:{switch(r=e,a=i,a){case 0:case 1:throw Error(s(345));case 4:if((t&4194048)!==t)break;case 6:Cu(r,t,eu,!Jl);break a;case 2:ru=null;break;case 3:case 5:break;default:throw Error(s(329))}if((t&62914560)===t&&(i=au+300-Fe(),10<i)){if(Cu(r,t,eu,!Jl),tt(r,0,!0)!==0)break a;fu=t,r.timeoutHandle=Yd(xu.bind(null,r,n,ru,cu,iu,t,eu,Ql,tu,Jl,a,`Throttled`,-0,0),i);break a}xu(r,n,ru,cu,iu,t,eu,Ql,tu,Jl,a,null,-0,0)}}break}while(1);od(e)}function xu(e,t,n,r,i,a,o,s,c,l,u,d,f,p){if(e.timeoutHandle=-1,d=t.subtreeFlags,d&8192||(d&16785408)==16785408){d={stylesheets:null,count:0,imgCount:0,imgBytes:0,suspenseyImages:[],waitingForImages:!0,waitingForViewTransition:!1,unsuspend:ln},Ll(t,a,d);var m=(a&62914560)===a?au-Fe():(a&4194048)===a?ou-Fe():0;if(m=Yf(d,m),m!==null){fu=a,e.cancelPendingCommit=m(Bu.bind(null,e,t,a,n,r,i,o,s,c,u,d,null,f,p)),Cu(e,a,o,!l);return}}Bu(e,t,a,n,r,i,o,s,c)}function Su(e){for(var t=e;;){var n=t.tag;if((n===0||n===11||n===15)&&t.flags&16384&&(n=t.updateQueue,n!==null&&(n=n.stores,n!==null)))for(var r=0;r<n.length;r++){var i=n[r],a=i.getSnapshot;i=i.value;try{if(!Ar(a(),i))return!1}catch{return!1}}if(n=t.child,t.subtreeFlags&16384&&n!==null)n.return=t,t=n;else{if(t===e)break;for(;t.sibling===null;){if(t.return===null||t.return===e)return!0;t=t.return}t.sibling.return=t.return,t=t.sibling}}return!0}function Cu(e,t,n,r){t&=~$l,t&=~Ql,e.suspendedLanes|=t,e.pingedLanes&=~t,r&&(e.warmLanes|=t),r=e.expirationTimes;for(var i=t;0<i;){var a=31-qe(i),o=1<<a;r[a]=-1,i&=~o}n!==0&&ct(e,n,t)}function wu(){return B&6?!0:(sd(0,!1),!1)}function Tu(){if(V!==null){if(Kl===0)var e=V.return;else e=V,ea=$i=null,Io(e),Ra=null,za=0,e=V;for(;e!==null;)Kc(e.alternate,e),e=e.return;V=null}}function Eu(e,t){var n=e.timeoutHandle;n!==-1&&(e.timeoutHandle=-1,Xd(n)),n=e.cancelPendingCommit,n!==null&&(e.cancelPendingCommit=null,n()),fu=0,Tu(),Gl=e,V=n=_i(e.current,null),H=t,Kl=0,ql=null,Jl=!1,Yl=nt(e,t),Xl=!1,tu=eu=$l=Ql=Zl=W=0,ru=nu=null,iu=!1,t&8&&(t|=t&32);var r=e.entangledLanes;if(r!==0)for(e=e.entanglements,r&=t;0<r;){var i=31-qe(r),a=1<<i;t|=e[i],r&=~a}return U=t,si(),n}function Du(e,t){F=null,D.H=Ws,t===Oa||t===Aa?(t=Ia(),Kl=3):t===ka?(t=Ia(),Kl=4):Kl=t===cc?8:typeof t==`object`&&t&&typeof t.then==`function`?6:1,ql=t,V===null&&(W=1,nc(e,Ti(t,e.current)))}function Ou(){var e=lo.current;return e===null?!0:(H&4194048)===H?uo===null:(H&62914560)===H||H&536870912?e===uo:!1}function ku(){var e=D.H;return D.H=Ws,e===null?Ws:e}function Au(){var e=D.A;return D.A=Ul,e}function ju(){W=4,Jl||(H&4194048)!==H&&lo.current!==null||(Yl=!0),!(Zl&134217727)&&!(Ql&134217727)||Gl===null||Cu(Gl,H,eu,!1)}function Mu(e,t,n){var r=B;B|=2;var i=ku(),a=Au();(Gl!==e||H!==t)&&(cu=null,Eu(e,t)),t=!1;var o=W;a:do try{if(Kl!==0&&V!==null){var s=V,c=ql;switch(Kl){case 8:Tu(),o=6;break a;case 3:case 2:case 9:case 6:lo.current===null&&(t=!0);var l=Kl;if(Kl=0,ql=null,Lu(e,s,c,l),n&&Yl){o=0;break a}break;default:l=Kl,Kl=0,ql=null,Lu(e,s,c,l)}}Nu(),o=W;break}catch(t){Du(e,t)}while(1);return t&&e.shellSuspendCounter++,ea=$i=null,B=r,D.H=i,D.A=a,V===null&&(Gl=null,H=0,si()),o}function Nu(){for(;V!==null;)Fu(V)}function Pu(e,t){var n=B;B|=2;var r=ku(),i=Au();Gl!==e||H!==t?(cu=null,su=Fe()+500,Eu(e,t)):Yl=nt(e,t);a:do try{if(Kl!==0&&V!==null){t=V;var a=ql;b:switch(Kl){case 1:Kl=0,ql=null,Lu(e,t,a,1);break;case 2:case 9:if(Ma(a)){Kl=0,ql=null,Iu(t);break}t=function(){Kl!==2&&Kl!==9||Gl!==e||(Kl=7),od(e)},a.then(t,t);break a;case 3:Kl=7;break a;case 4:Kl=5;break a;case 7:Ma(a)?(Kl=0,ql=null,Iu(t)):(Kl=0,ql=null,Lu(e,t,a,7));break;case 5:var o=null;switch(V.tag){case 26:o=V.memoizedState;case 5:case 27:var c=V;if(o?Kf(o):c.stateNode.complete){Kl=0,ql=null;var l=c.sibling;if(l!==null)V=l;else{var u=c.return;u===null?V=null:(V=u,Ru(u))}break b}}Kl=0,ql=null,Lu(e,t,a,5);break;case 6:Kl=0,ql=null,Lu(e,t,a,6);break;case 8:Tu(),W=6;break a;default:throw Error(s(462))}}q();break}catch(t){Du(e,t)}while(1);return ea=$i=null,D.H=r,D.A=i,B=n,V===null?(Gl=null,H=0,si(),W):0}function q(){for(;V!==null&&!Ne();)Fu(V)}function Fu(e){var t=R(e.alternate,e,U);e.memoizedProps=e.pendingProps,t===null?Ru(e):V=t}function Iu(e){var t=e,n=t.alternate;switch(t.tag){case 15:case 0:t=Sc(n,t,t.pendingProps,t.type,void 0,H);break;case 11:t=Sc(n,t,t.pendingProps,t.type.render,t.ref,H);break;case 5:Io(t);default:Kc(n,t),t=V=vi(t,U),t=R(n,t,U)}e.memoizedProps=e.pendingProps,t===null?Ru(e):V=t}function Lu(e,t,n,r){ea=$i=null,Io(t),Ra=null,za=0;var i=t.return;try{if(sc(e,i,t,n,H)){W=1,nc(e,Ti(n,e.current)),V=null;return}}catch(t){if(i!==null)throw V=i,t;W=1,nc(e,Ti(n,e.current)),V=null;return}t.flags&32768?(M||r===1?e=!0:Yl||H&536870912?e=!1:(Jl=e=!0,(r===2||r===9||r===3||r===6)&&(r=lo.current,r!==null&&r.tag===13&&(r.flags|=16384))),zu(t,e)):Ru(t)}function Ru(e){var t=e;do{if(t.flags&32768){zu(t,Jl);return}e=t.return;var n=Wc(t.alternate,t,U);if(n!==null){V=n;return}if(t=t.sibling,t!==null){V=t;return}V=t=e}while(t!==null);W===0&&(W=5)}function zu(e,t){do{var n=Gc(e.alternate,e);if(n!==null){n.flags&=32767,V=n;return}if(n=e.return,n!==null&&(n.flags|=32768,n.subtreeFlags=0,n.deletions=null),!t&&(e=e.sibling,e!==null)){V=e;return}V=e=n}while(e!==null);W=6,V=null}function Bu(e,t,n,r,i,a,o,c,l){e.cancelPendingCommit=null;do Gu();while(lu!==0);if(B&6)throw Error(s(327));if(t!==null){if(t===e.current)throw Error(s(177));if(a=t.lanes|t.childLanes,a|=oi,st(e,n,a,o,c,l),e===Gl&&(V=Gl=null,H=0),du=t,uu=e,fu=n,pu=a,mu=i,hu=r,t.subtreeFlags&10256||t.flags&10256?(e.callbackNode=null,e.callbackPriority=0,$u(ze,function(){return Ku(),null})):(e.callbackNode=null,e.callbackPriority=0),r=(t.flags&13878)!=0,t.subtreeFlags&13878||r){r=D.T,D.T=null,i=O.p,O.p=2,o=B,B|=4;try{dl(e,t,n)}finally{B=o,O.p=i,D.T=r}}lu=1,Vu(),Hu(),Uu()}}function Vu(){if(lu===1){lu=0;var e=uu,t=du,n=(t.flags&13878)!=0;if(t.subtreeFlags&13878||n){n=D.T,D.T=null;var r=O.p;O.p=2;var i=B;B|=4;try{wl(t,e);var a=Hd,o=Fr(e.containerInfo),s=a.focusedElem,c=a.selectionRange;if(o!==s&&s&&s.ownerDocument&&Pr(s.ownerDocument.documentElement,s)){if(c!==null&&Ir(s)){var l=c.start,u=c.end;if(u===void 0&&(u=l),`selectionStart`in s)s.selectionStart=l,s.selectionEnd=Math.min(u,s.value.length);else{var d=s.ownerDocument||document,f=d&&d.defaultView||window;if(f.getSelection){var p=f.getSelection(),m=s.textContent.length,h=Math.min(c.start,m),g=c.end===void 0?h:Math.min(c.end,m);!p.extend&&h>g&&(o=g,g=h,h=o);var _=Nr(s,h),v=Nr(s,g);if(_&&v&&(p.rangeCount!==1||p.anchorNode!==_.node||p.anchorOffset!==_.offset||p.focusNode!==v.node||p.focusOffset!==v.offset)){var y=d.createRange();y.setStart(_.node,_.offset),p.removeAllRanges(),h>g?(p.addRange(y),p.extend(v.node,v.offset)):(y.setEnd(v.node,v.offset),p.addRange(y))}}}}for(d=[],p=s;p=p.parentNode;)p.nodeType===1&&d.push({element:p,left:p.scrollLeft,top:p.scrollTop});for(typeof s.focus==`function`&&s.focus(),s=0;s<d.length;s++){var b=d[s];b.element.scrollLeft=b.left,b.element.scrollTop=b.top}}lp=!!X,Hd=X=null}finally{B=i,O.p=r,D.T=n}}e.current=t,lu=2}}function Hu(){if(lu===2){lu=0;var e=uu,t=du,n=(t.flags&8772)!=0;if(t.subtreeFlags&8772||n){n=D.T,D.T=null;var r=O.p;O.p=2;var i=B;B|=4;try{fl(e,t.alternate,t)}finally{B=i,O.p=r,D.T=n}}lu=3}}function Uu(){if(lu===4||lu===3){lu=0,Pe();var e=uu,t=du,n=fu,r=hu;t.subtreeFlags&10256||t.flags&10256?lu=5:(lu=0,du=uu=null,Wu(e,e.pendingLanes));var i=e.pendingLanes;if(i===0&&(G=null),ft(n),t=t.stateNode,Ge&&typeof Ge.onCommitFiberRoot==`function`)try{Ge.onCommitFiberRoot(We,t,void 0,(t.current.flags&128)==128)}catch{}if(r!==null){t=D.T,i=O.p,O.p=2,D.T=null;try{for(var a=e.onRecoverableError,o=0;o<r.length;o++){var s=r[o];a(s.value,{componentStack:s.stack})}}finally{D.T=t,O.p=i}}fu&3&&Gu(),od(e),i=e.pendingLanes,n&261930&&i&42?e===_u?gu++:(gu=0,_u=e):gu=0,sd(0,!1)}}function Wu(e,t){(e.pooledCacheLanes&=t)===0&&(t=e.pooledCache,t!=null&&(e.pooledCache=null,ha(t)))}function Gu(){return Vu(),Hu(),Uu(),Ku()}function Ku(){if(lu!==5)return!1;var e=uu,t=pu;pu=0;var n=ft(fu),r=D.T,i=O.p;try{O.p=32>n?32:n,D.T=null,n=mu,mu=null;var a=uu,o=fu;if(lu=0,du=uu=null,fu=0,B&6)throw Error(s(331));var c=B;if(B|=4,Bl(a.current),Ml(a,a.current,o,n),B=c,sd(0,!1),Ge&&typeof Ge.onPostCommitFiberRoot==`function`)try{Ge.onPostCommitFiberRoot(We,a)}catch{}return!0}finally{O.p=i,D.T=r,Wu(e,t)}}function qu(e,t,n){t=Ti(n,t),t=ic(e.stateNode,t,2),e=Xa(e,t,2),e!==null&&(ot(e,2),od(e))}function J(e,t,n){if(e.tag===3)qu(e,e,n);else for(;t!==null;){if(t.tag===3){qu(t,e,n);break}else if(t.tag===1){var r=t.stateNode;if(typeof t.type.getDerivedStateFromError==`function`||typeof r.componentDidCatch==`function`&&(G===null||!G.has(r))){e=Ti(n,e),n=ac(2),r=Xa(t,n,2),r!==null&&(oc(n,r,t,e),ot(r,2),od(r));break}}t=t.return}}function Ju(e,t,n){var r=e.pingCache;if(r===null){r=e.pingCache=new Wl;var i=new Set;r.set(t,i)}else i=r.get(t),i===void 0&&(i=new Set,r.set(t,i));i.has(n)||(Xl=!0,i.add(n),e=Yu.bind(null,e,t,n),t.then(e,e))}function Yu(e,t,n){var r=e.pingCache;r!==null&&r.delete(t),e.pingedLanes|=e.suspendedLanes&n,e.warmLanes&=~n,Gl===e&&(H&n)===n&&(W===4||W===3&&(H&62914560)===H&&300>Fe()-au?!(B&2)&&Eu(e,0):$l|=n,tu===H&&(tu=0)),od(e)}function Xu(e,t){t===0&&(t=it()),e=ui(e,t),e!==null&&(ot(e,t),od(e))}function Zu(e){var t=e.memoizedState,n=0;t!==null&&(n=t.retryLane),Xu(e,n)}function Qu(e,t){var n=0;switch(e.tag){case 31:case 13:var r=e.stateNode,i=e.memoizedState;i!==null&&(n=i.retryLane);break;case 19:r=e.stateNode;break;case 22:r=e.stateNode._retryCache;break;default:throw Error(s(314))}r!==null&&r.delete(t),Xu(e,n)}function $u(e,t){return je(e,t)}var ed=null,td=null,nd=!1,rd=!1,id=!1,ad=0;function od(e){e!==td&&e.next===null&&(td===null?ed=td=e:td=td.next=e),rd=!0,nd||(nd=!0,pd())}function sd(e,t){if(!id&&rd){id=!0;do for(var n=!1,r=ed;r!==null;){if(!t)if(e!==0){var i=r.pendingLanes;if(i===0)var a=0;else{var o=r.suspendedLanes,s=r.pingedLanes;a=(1<<31-qe(42|e)+1)-1,a&=i&~(o&~s),a=a&201326741?a&201326741|1:a?a|2:0}a!==0&&(n=!0,fd(r,a))}else a=H,a=tt(r,r===Gl?a:0,r.cancelPendingCommit!==null||r.timeoutHandle!==-1),!(a&3)||nt(r,a)||(n=!0,fd(r,a));r=r.next}while(n);id=!1}}function cd(){ld()}function ld(){rd=nd=!1;var e=0;ad!==0&&Jd()&&(e=ad);for(var t=Fe(),n=null,r=ed;r!==null;){var i=r.next,a=ud(r,t);a===0?(r.next=null,n===null?ed=i:n.next=i,i===null&&(td=n)):(n=r,(e!==0||a&3)&&(rd=!0)),r=i}lu!==0&&lu!==5||sd(e,!1),ad!==0&&(ad=0)}function ud(e,t){for(var n=e.suspendedLanes,r=e.pingedLanes,i=e.expirationTimes,a=e.pendingLanes&-62914561;0<a;){var o=31-qe(a),s=1<<o,c=i[o];c===-1?((s&n)===0||(s&r)!==0)&&(i[o]=rt(s,t)):c<=t&&(e.expiredLanes|=s),a&=~s}if(t=Gl,n=H,n=tt(e,e===t?n:0,e.cancelPendingCommit!==null||e.timeoutHandle!==-1),r=e.callbackNode,n===0||e===t&&(Kl===2||Kl===9)||e.cancelPendingCommit!==null)return r!==null&&r!==null&&Me(r),e.callbackNode=null,e.callbackPriority=0;if(!(n&3)||nt(e,n)){if(t=n&-n,t===e.callbackPriority)return t;switch(r!==null&&Me(r),ft(n)){case 2:case 8:n=Re;break;case 32:n=ze;break;case 268435456:n=Ve;break;default:n=ze}return r=dd.bind(null,e),n=je(n,r),e.callbackPriority=t,e.callbackNode=n,t}return r!==null&&r!==null&&Me(r),e.callbackPriority=2,e.callbackNode=null,2}function dd(e,t){if(lu!==0&&lu!==5)return e.callbackNode=null,e.callbackPriority=0,null;var n=e.callbackNode;if(Gu()&&e.callbackNode!==n)return null;var r=H;return r=tt(e,e===Gl?r:0,e.cancelPendingCommit!==null||e.timeoutHandle!==-1),r===0?null:(bu(e,r,t),ud(e,Fe()),e.callbackNode!=null&&e.callbackNode===n?dd.bind(null,e):null)}function fd(e,t){if(Gu())return null;bu(e,t,!0)}function pd(){Qd(function(){B&6?je(Le,cd):ld()})}function md(){if(ad===0){var e=va;e===0&&(e=Ze,Ze<<=1,!(Ze&261888)&&(Ze=256)),ad=e}return ad}function hd(e){return e==null||typeof e==`symbol`||typeof e==`boolean`?null:typeof e==`function`?e:cn(``+e)}function gd(e,t){var n=t.ownerDocument.createElement(`input`);return n.name=t.name,n.value=t.value,e.id&&n.setAttribute(`form`,e.id),t.parentNode.insertBefore(n,t),e=new FormData(e),n.parentNode.removeChild(n),e}function _d(e,t,n,r,i){if(t===`submit`&&n&&n.stateNode===i){var a=hd((i[_t]||null).action),o=r.submitter;o&&(t=(t=o[_t]||null)?hd(t.formAction):o.getAttribute(`formAction`),t!==null&&(a=t,o=null));var s=new An(`action`,`action`,null,r,i);e.push({event:s,listeners:[{instance:null,listener:function(){if(r.defaultPrevented){if(ad!==0){var e=o?gd(i,o):new FormData(i);As(n,{pending:!0,data:e,method:i.method,action:a},null,e)}}else typeof a==`function`&&(s.preventDefault(),e=o?gd(i,o):new FormData(i),As(n,{pending:!0,data:e,method:i.method,action:a},a,e))},currentTarget:i}]})}}for(var vd=0;vd<ti.length;vd++){var yd=ti[vd];ni(yd.toLowerCase(),`on`+(yd[0].toUpperCase()+yd.slice(1)))}ni(qr,`onAnimationEnd`),ni(Jr,`onAnimationIteration`),ni(Yr,`onAnimationStart`),ni(`dblclick`,`onDoubleClick`),ni(`focusin`,`onFocus`),ni(`focusout`,`onBlur`),ni(Xr,`onTransitionRun`),ni(Zr,`onTransitionStart`),ni(Qr,`onTransitionCancel`),ni($r,`onTransitionEnd`),Nt(`onMouseEnter`,[`mouseout`,`mouseover`]),Nt(`onMouseLeave`,[`mouseout`,`mouseover`]),Nt(`onPointerEnter`,[`pointerout`,`pointerover`]),Nt(`onPointerLeave`,[`pointerout`,`pointerover`]),Mt(`onChange`,`change click focusin focusout input keydown keyup selectionchange`.split(` `)),Mt(`onSelect`,`focusout contextmenu dragend focusin keydown keyup mousedown mouseup selectionchange`.split(` `)),Mt(`onBeforeInput`,[`compositionend`,`keypress`,`textInput`,`paste`]),Mt(`onCompositionEnd`,`compositionend focusout keydown keypress keyup mousedown`.split(` `)),Mt(`onCompositionStart`,`compositionstart focusout keydown keypress keyup mousedown`.split(` `)),Mt(`onCompositionUpdate`,`compositionupdate focusout keydown keypress keyup mousedown`.split(` `));var bd=`abort canplay canplaythrough durationchange emptied encrypted ended error loadeddata loadedmetadata loadstart pause play playing progress ratechange resize seeked seeking stalled suspend timeupdate volumechange waiting`.split(` `),xd=new Set(`beforetoggle cancel close invalid load scroll scrollend toggle`.split(` `).concat(bd));function Sd(e,t){t=(t&4)!=0;for(var n=0;n<e.length;n++){var r=e[n],i=r.event;r=r.listeners;a:{var a=void 0;if(t)for(var o=r.length-1;0<=o;o--){var s=r[o],c=s.instance,l=s.currentTarget;if(s=s.listener,c!==a&&i.isPropagationStopped())break a;a=s,i.currentTarget=l;try{a(i)}catch(e){ri(e)}i.currentTarget=null,a=c}else for(o=0;o<r.length;o++){if(s=r[o],c=s.instance,l=s.currentTarget,s=s.listener,c!==a&&i.isPropagationStopped())break a;a=s,i.currentTarget=l;try{a(i)}catch(e){ri(e)}i.currentTarget=null,a=c}}}}function Y(e,t){var n=t[yt];n===void 0&&(n=t[yt]=new Set);var r=e+`__bubble`;n.has(r)||(Ed(t,e,2,!1),n.add(r))}function Cd(e,t,n){var r=0;t&&(r|=4),Ed(n,e,r,t)}var wd=`_reactListening`+Math.random().toString(36).slice(2);function Td(e){if(!e[wd]){e[wd]=!0,At.forEach(function(t){t!==`selectionchange`&&(xd.has(t)||Cd(t,!1,e),Cd(t,!0,e))});var t=e.nodeType===9?e:e.ownerDocument;t===null||t[wd]||(t[wd]=!0,Cd(`selectionchange`,!1,t))}}function Ed(e,t,n,r){switch(gp(t)){case 2:var i=up;break;case 8:i=dp;break;default:i=fp}n=i.bind(null,t,n,e),i=void 0,!yn||t!==`touchstart`&&t!==`touchmove`&&t!==`wheel`||(i=!0),r?i===void 0?e.addEventListener(t,n,!0):e.addEventListener(t,n,{capture:!0,passive:i}):i===void 0?e.addEventListener(t,n,!1):e.addEventListener(t,n,{passive:i})}function Dd(e,t,n,r,i){var a=r;if(!(t&1)&&!(t&2)&&r!==null)a:for(;;){if(r===null)return;var o=r.tag;if(o===3||o===4){var s=r.stateNode.containerInfo;if(s===i)break;if(o===4)for(o=r.return;o!==null;){var c=o.tag;if((c===3||c===4)&&o.stateNode.containerInfo===i)return;o=o.return}for(;s!==null;){if(o=Tt(s),o===null)return;if(c=o.tag,c===5||c===6||c===26||c===27){r=a=o;continue a}s=s.parentNode}}r=r.return}gn(function(){var r=a,i=dn(n),o=[];a:{var s=ei.get(e);if(s!==void 0){var c=An,u=e;switch(e){case`keypress`:if(Tn(n)===0)break a;case`keydown`:case`keyup`:c=Jn;break;case`focusin`:u=`focus`,c=zn;break;case`focusout`:u=`blur`,c=zn;break;case`beforeblur`:case`afterblur`:c=zn;break;case`click`:if(n.button===2)break a;case`auxclick`:case`dblclick`:case`mousedown`:case`mousemove`:case`mouseup`:case`mouseout`:case`mouseover`:case`contextmenu`:c=Ln;break;case`drag`:case`dragend`:case`dragenter`:case`dragexit`:case`dragleave`:case`dragover`:case`dragstart`:case`drop`:c=Rn;break;case`touchcancel`:case`touchend`:case`touchmove`:case`touchstart`:c=Xn;break;case qr:case Jr:case Yr:c=Bn;break;case $r:c=Zn;break;case`scroll`:case`scrollend`:c=Mn;break;case`wheel`:c=Qn;break;case`copy`:case`cut`:case`paste`:c=Vn;break;case`gotpointercapture`:case`lostpointercapture`:case`pointercancel`:case`pointerdown`:case`pointermove`:case`pointerout`:case`pointerover`:case`pointerup`:c=Yn;break;case`toggle`:case`beforetoggle`:c=$n}var d=(t&4)!=0,f=!d&&(e===`scroll`||e===`scrollend`),p=d?s===null?null:s+`Capture`:s;d=[];for(var m=r,h;m!==null;){var g=m;if(h=g.stateNode,g=g.tag,g!==5&&g!==26&&g!==27||h===null||p===null||(g=_n(m,p),g!=null&&d.push(Od(m,g,h))),f)break;m=m.return}0<d.length&&(s=new c(s,u,null,n,i),o.push({event:s,listeners:d}))}}if(!(t&7)){a:{if(s=e===`mouseover`||e===`pointerover`,c=e===`mouseout`||e===`pointerout`,s&&n!==un&&(u=n.relatedTarget||n.fromElement)&&(Tt(u)||u[vt]))break a;if((c||s)&&(s=i.window===i?i:(s=i.ownerDocument)?s.defaultView||s.parentWindow:window,c?(u=n.relatedTarget||n.toElement,c=r,u=u?Tt(u):null,u!==null&&(f=l(u),d=u.tag,u!==f||d!==5&&d!==27&&d!==6)&&(u=null)):(c=null,u=r),c!==u)){if(d=Ln,g=`onMouseLeave`,p=`onMouseEnter`,m=`mouse`,(e===`pointerout`||e===`pointerover`)&&(d=Yn,g=`onPointerLeave`,p=`onPointerEnter`,m=`pointer`),f=c==null?s:Dt(c),h=u==null?s:Dt(u),s=new d(g,m+`leave`,c,n,i),s.target=f,s.relatedTarget=h,g=null,Tt(i)===r&&(d=new d(p,m+`enter`,u,n,i),d.target=h,d.relatedTarget=f,g=d),f=g,c&&u)b:{for(d=Ad,p=c,m=u,h=0,g=p;g;g=d(g))h++;g=0;for(var _=m;_;_=d(_))g++;for(;0<h-g;)p=d(p),h--;for(;0<g-h;)m=d(m),g--;for(;h--;){if(p===m||m!==null&&p===m.alternate){d=p;break b}p=d(p),m=d(m)}d=null}else d=null;c!==null&&jd(o,s,c,d,!1),u!==null&&f!==null&&jd(o,f,u,d,!0)}}a:{if(s=r?Dt(r):window,c=s.nodeName&&s.nodeName.toLowerCase(),c===`select`||c===`input`&&s.type===`file`)var v=yr;else if(pr(s))if(br)v=Or;else{v=Er;var y=Tr}else c=s.nodeName,!c||c.toLowerCase()!==`input`||s.type!==`checkbox`&&s.type!==`radio`?r&&an(r.elementType)&&(v=yr):v=Dr;if(v&&=v(e,r)){mr(o,v,n,i);break a}y&&y(e,s,r),e===`focusout`&&r&&s.type===`number`&&r.memoizedProps.value!=null&&Zt(s,`number`,s.value)}switch(y=r?Dt(r):window,e){case`focusin`:(pr(y)||y.contentEditable===`true`)&&(Rr=y,zr=r,Br=null);break;case`focusout`:Br=zr=Rr=null;break;case`mousedown`:Vr=!0;break;case`contextmenu`:case`mouseup`:case`dragend`:Vr=!1,Hr(o,n,i);break;case`selectionchange`:if(Lr)break;case`keydown`:case`keyup`:Hr(o,n,i)}var b;if(tr)b:{switch(e){case`compositionstart`:var x=`onCompositionStart`;break b;case`compositionend`:x=`onCompositionEnd`;break b;case`compositionupdate`:x=`onCompositionUpdate`;break b}x=void 0}else lr?sr(e,n)&&(x=`onCompositionEnd`):e===`keydown`&&n.keyCode===229&&(x=`onCompositionStart`);x&&(ir&&n.locale!==`ko`&&(lr||x!==`onCompositionStart`?x===`onCompositionEnd`&&lr&&(b=wn()):(xn=i,Sn=`value`in xn?xn.value:xn.textContent,lr=!0)),y=kd(r,x),0<y.length&&(x=new Hn(x,e,null,n,i),o.push({event:x,listeners:y}),b?x.data=b:(b=cr(n),b!==null&&(x.data=b)))),(b=rr?ur(e,n):dr(e,n))&&(x=kd(r,`onBeforeInput`),0<x.length&&(y=new Hn(`onBeforeInput`,`beforeinput`,null,n,i),o.push({event:y,listeners:x}),y.data=b)),_d(o,e,r,n,i)}Sd(o,t)})}function Od(e,t,n){return{instance:e,listener:t,currentTarget:n}}function kd(e,t){for(var n=t+`Capture`,r=[];e!==null;){var i=e,a=i.stateNode;if(i=i.tag,i!==5&&i!==26&&i!==27||a===null||(i=_n(e,n),i!=null&&r.unshift(Od(e,i,a)),i=_n(e,t),i!=null&&r.push(Od(e,i,a))),e.tag===3)return r;e=e.return}return[]}function Ad(e){if(e===null)return null;do e=e.return;while(e&&e.tag!==5&&e.tag!==27);return e||null}function jd(e,t,n,r,i){for(var a=t._reactName,o=[];n!==null&&n!==r;){var s=n,c=s.alternate,l=s.stateNode;if(s=s.tag,c!==null&&c===r)break;s!==5&&s!==26&&s!==27||l===null||(c=l,i?(l=_n(n,a),l!=null&&o.unshift(Od(n,l,c))):i||(l=_n(n,a),l!=null&&o.push(Od(n,l,c)))),n=n.return}o.length!==0&&e.push({event:t,listeners:o})}var Md=/\r\n?/g,Nd=/\u0000|\uFFFD/g;function Pd(e){return(typeof e==`string`?e:``+e).replace(Md,`
`).replace(Nd,``)}function Fd(e,t){return t=Pd(t),Pd(e)===t}function Id(e,t,n,r,i,a){switch(n){case`children`:typeof r==`string`?t===`body`||t===`textarea`&&r===``||k(e,r):(typeof r==`number`||typeof r==`bigint`)&&t!==`body`&&k(e,``+r);break;case`className`:zt(e,`class`,r);break;case`tabIndex`:zt(e,`tabindex`,r);break;case`dir`:case`role`:case`viewBox`:case`width`:case`height`:zt(e,n,r);break;case`style`:rn(e,r,a);break;case`data`:if(t!==`object`){zt(e,`data`,r);break}case`src`:case`href`:if(r===``&&(t!==`a`||n!==`href`)){e.removeAttribute(n);break}if(r==null||typeof r==`function`||typeof r==`symbol`||typeof r==`boolean`){e.removeAttribute(n);break}r=cn(``+r),e.setAttribute(n,r);break;case`action`:case`formAction`:if(typeof r==`function`){e.setAttribute(n,`javascript:throw new Error('A React form was unexpectedly submitted. If you called form.submit() manually, consider using form.requestSubmit() instead. If you\\'re trying to use event.stopPropagation() in a submit event handler, consider also calling event.preventDefault().')`);break}else typeof a==`function`&&(n===`formAction`?(t!==`input`&&Id(e,t,`name`,i.name,i,null),Id(e,t,`formEncType`,i.formEncType,i,null),Id(e,t,`formMethod`,i.formMethod,i,null),Id(e,t,`formTarget`,i.formTarget,i,null)):(Id(e,t,`encType`,i.encType,i,null),Id(e,t,`method`,i.method,i,null),Id(e,t,`target`,i.target,i,null)));if(r==null||typeof r==`symbol`||typeof r==`boolean`){e.removeAttribute(n);break}r=cn(``+r),e.setAttribute(n,r);break;case`onClick`:r!=null&&(e.onclick=ln);break;case`onScroll`:r!=null&&Y(`scroll`,e);break;case`onScrollEnd`:r!=null&&Y(`scrollend`,e);break;case`dangerouslySetInnerHTML`:if(r!=null){if(typeof r!=`object`||!(`__html`in r))throw Error(s(61));if(n=r.__html,n!=null){if(i.children!=null)throw Error(s(60));e.innerHTML=n}}break;case`multiple`:e.multiple=r&&typeof r!=`function`&&typeof r!=`symbol`;break;case`muted`:e.muted=r&&typeof r!=`function`&&typeof r!=`symbol`;break;case`suppressContentEditableWarning`:case`suppressHydrationWarning`:case`defaultValue`:case`defaultChecked`:case`innerHTML`:case`ref`:break;case`autoFocus`:break;case`xlinkHref`:if(r==null||typeof r==`function`||typeof r==`boolean`||typeof r==`symbol`){e.removeAttribute(`xlink:href`);break}n=cn(``+r),e.setAttributeNS(`http://www.w3.org/1999/xlink`,`xlink:href`,n);break;case`contentEditable`:case`spellCheck`:case`draggable`:case`value`:case`autoReverse`:case`externalResourcesRequired`:case`focusable`:case`preserveAlpha`:r!=null&&typeof r!=`function`&&typeof r!=`symbol`?e.setAttribute(n,``+r):e.removeAttribute(n);break;case`inert`:case`allowFullScreen`:case`async`:case`autoPlay`:case`controls`:case`default`:case`defer`:case`disabled`:case`disablePictureInPicture`:case`disableRemotePlayback`:case`formNoValidate`:case`hidden`:case`loop`:case`noModule`:case`noValidate`:case`open`:case`playsInline`:case`readOnly`:case`required`:case`reversed`:case`scoped`:case`seamless`:case`itemScope`:r&&typeof r!=`function`&&typeof r!=`symbol`?e.setAttribute(n,``):e.removeAttribute(n);break;case`capture`:case`download`:!0===r?e.setAttribute(n,``):!1!==r&&r!=null&&typeof r!=`function`&&typeof r!=`symbol`?e.setAttribute(n,r):e.removeAttribute(n);break;case`cols`:case`rows`:case`size`:case`span`:r!=null&&typeof r!=`function`&&typeof r!=`symbol`&&!isNaN(r)&&1<=r?e.setAttribute(n,r):e.removeAttribute(n);break;case`rowSpan`:case`start`:r==null||typeof r==`function`||typeof r==`symbol`||isNaN(r)?e.removeAttribute(n):e.setAttribute(n,r);break;case`popover`:Y(`beforetoggle`,e),Y(`toggle`,e),Rt(e,`popover`,r);break;case`xlinkActuate`:Bt(e,`http://www.w3.org/1999/xlink`,`xlink:actuate`,r);break;case`xlinkArcrole`:Bt(e,`http://www.w3.org/1999/xlink`,`xlink:arcrole`,r);break;case`xlinkRole`:Bt(e,`http://www.w3.org/1999/xlink`,`xlink:role`,r);break;case`xlinkShow`:Bt(e,`http://www.w3.org/1999/xlink`,`xlink:show`,r);break;case`xlinkTitle`:Bt(e,`http://www.w3.org/1999/xlink`,`xlink:title`,r);break;case`xlinkType`:Bt(e,`http://www.w3.org/1999/xlink`,`xlink:type`,r);break;case`xmlBase`:Bt(e,`http://www.w3.org/XML/1998/namespace`,`xml:base`,r);break;case`xmlLang`:Bt(e,`http://www.w3.org/XML/1998/namespace`,`xml:lang`,r);break;case`xmlSpace`:Bt(e,`http://www.w3.org/XML/1998/namespace`,`xml:space`,r);break;case`is`:Rt(e,`is`,r);break;case`innerText`:case`textContent`:break;default:(!(2<n.length)||n[0]!==`o`&&n[0]!==`O`||n[1]!==`n`&&n[1]!==`N`)&&(n=on.get(n)||n,Rt(e,n,r))}}function Ld(e,t,n,r,i,a){switch(n){case`style`:rn(e,r,a);break;case`dangerouslySetInnerHTML`:if(r!=null){if(typeof r!=`object`||!(`__html`in r))throw Error(s(61));if(n=r.__html,n!=null){if(i.children!=null)throw Error(s(60));e.innerHTML=n}}break;case`children`:typeof r==`string`?k(e,r):(typeof r==`number`||typeof r==`bigint`)&&k(e,``+r);break;case`onScroll`:r!=null&&Y(`scroll`,e);break;case`onScrollEnd`:r!=null&&Y(`scrollend`,e);break;case`onClick`:r!=null&&(e.onclick=ln);break;case`suppressContentEditableWarning`:case`suppressHydrationWarning`:case`innerHTML`:case`ref`:break;case`innerText`:case`textContent`:break;default:if(!jt.hasOwnProperty(n))a:{if(n[0]===`o`&&n[1]===`n`&&(i=n.endsWith(`Capture`),t=n.slice(2,i?n.length-7:void 0),a=e[_t]||null,a=a==null?null:a[n],typeof a==`function`&&e.removeEventListener(t,a,i),typeof r==`function`)){typeof a!=`function`&&a!==null&&(n in e?e[n]=null:e.hasAttribute(n)&&e.removeAttribute(n)),e.addEventListener(t,r,i);break a}n in e?e[n]=r:!0===r?e.setAttribute(n,``):Rt(e,n,r)}}}function Rd(e,t,n){switch(t){case`div`:case`span`:case`svg`:case`path`:case`a`:case`g`:case`p`:case`li`:break;case`img`:Y(`error`,e),Y(`load`,e);var r=!1,i=!1,a;for(a in n)if(n.hasOwnProperty(a)){var o=n[a];if(o!=null)switch(a){case`src`:r=!0;break;case`srcSet`:i=!0;break;case`children`:case`dangerouslySetInnerHTML`:throw Error(s(137,t));default:Id(e,t,a,o,n,null)}}i&&Id(e,t,`srcSet`,n.srcSet,n,null),r&&Id(e,t,`src`,n.src,n,null);return;case`input`:Y(`invalid`,e);var c=a=o=i=null,l=null,u=null;for(r in n)if(n.hasOwnProperty(r)){var d=n[r];if(d!=null)switch(r){case`name`:i=d;break;case`type`:o=d;break;case`checked`:l=d;break;case`defaultChecked`:u=d;break;case`value`:a=d;break;case`defaultValue`:c=d;break;case`children`:case`dangerouslySetInnerHTML`:if(d!=null)throw Error(s(137,t));break;default:Id(e,t,r,d,n,null)}}Xt(e,a,c,l,u,o,i,!1);return;case`select`:for(i in Y(`invalid`,e),r=o=a=null,n)if(n.hasOwnProperty(i)&&(c=n[i],c!=null))switch(i){case`value`:a=c;break;case`defaultValue`:o=c;break;case`multiple`:r=c;default:Id(e,t,i,c,n,null)}t=a,n=o,e.multiple=!!r,t==null?n!=null&&Qt(e,!!r,n,!0):Qt(e,!!r,t,!1);return;case`textarea`:for(o in Y(`invalid`,e),a=i=r=null,n)if(n.hasOwnProperty(o)&&(c=n[o],c!=null))switch(o){case`value`:r=c;break;case`defaultValue`:i=c;break;case`children`:a=c;break;case`dangerouslySetInnerHTML`:if(c!=null)throw Error(s(91));break;default:Id(e,t,o,c,n,null)}en(e,r,i,a);return;case`option`:for(l in n)if(n.hasOwnProperty(l)&&(r=n[l],r!=null))switch(l){case`selected`:e.selected=r&&typeof r!=`function`&&typeof r!=`symbol`;break;default:Id(e,t,l,r,n,null)}return;case`dialog`:Y(`beforetoggle`,e),Y(`toggle`,e),Y(`cancel`,e),Y(`close`,e);break;case`iframe`:case`object`:Y(`load`,e);break;case`video`:case`audio`:for(r=0;r<bd.length;r++)Y(bd[r],e);break;case`image`:Y(`error`,e),Y(`load`,e);break;case`details`:Y(`toggle`,e);break;case`embed`:case`source`:case`link`:Y(`error`,e),Y(`load`,e);case`area`:case`base`:case`br`:case`col`:case`hr`:case`keygen`:case`meta`:case`param`:case`track`:case`wbr`:case`menuitem`:for(u in n)if(n.hasOwnProperty(u)&&(r=n[u],r!=null))switch(u){case`children`:case`dangerouslySetInnerHTML`:throw Error(s(137,t));default:Id(e,t,u,r,n,null)}return;default:if(an(t)){for(d in n)n.hasOwnProperty(d)&&(r=n[d],r!==void 0&&Ld(e,t,d,r,n,void 0));return}}for(c in n)n.hasOwnProperty(c)&&(r=n[c],r!=null&&Id(e,t,c,r,n,null))}function zd(e,t,n,r){switch(t){case`div`:case`span`:case`svg`:case`path`:case`a`:case`g`:case`p`:case`li`:break;case`input`:var i=null,a=null,o=null,c=null,l=null,u=null,d=null;for(m in n){var f=n[m];if(n.hasOwnProperty(m)&&f!=null)switch(m){case`checked`:break;case`value`:break;case`defaultValue`:l=f;default:r.hasOwnProperty(m)||Id(e,t,m,null,r,f)}}for(var p in r){var m=r[p];if(f=n[p],r.hasOwnProperty(p)&&(m!=null||f!=null))switch(p){case`type`:a=m;break;case`name`:i=m;break;case`checked`:u=m;break;case`defaultChecked`:d=m;break;case`value`:o=m;break;case`defaultValue`:c=m;break;case`children`:case`dangerouslySetInnerHTML`:if(m!=null)throw Error(s(137,t));break;default:m!==f&&Id(e,t,p,m,r,f)}}Yt(e,o,c,l,u,d,a,i);return;case`select`:for(a in m=o=c=p=null,n)if(l=n[a],n.hasOwnProperty(a)&&l!=null)switch(a){case`value`:break;case`multiple`:m=l;default:r.hasOwnProperty(a)||Id(e,t,a,null,r,l)}for(i in r)if(a=r[i],l=n[i],r.hasOwnProperty(i)&&(a!=null||l!=null))switch(i){case`value`:p=a;break;case`defaultValue`:c=a;break;case`multiple`:o=a;default:a!==l&&Id(e,t,i,a,r,l)}t=c,n=o,r=m,p==null?!!r!=!!n&&(t==null?Qt(e,!!n,n?[]:``,!1):Qt(e,!!n,t,!0)):Qt(e,!!n,p,!1);return;case`textarea`:for(c in m=p=null,n)if(i=n[c],n.hasOwnProperty(c)&&i!=null&&!r.hasOwnProperty(c))switch(c){case`value`:break;case`children`:break;default:Id(e,t,c,null,r,i)}for(o in r)if(i=r[o],a=n[o],r.hasOwnProperty(o)&&(i!=null||a!=null))switch(o){case`value`:p=i;break;case`defaultValue`:m=i;break;case`children`:break;case`dangerouslySetInnerHTML`:if(i!=null)throw Error(s(91));break;default:i!==a&&Id(e,t,o,i,r,a)}$t(e,p,m);return;case`option`:for(var h in n)if(p=n[h],n.hasOwnProperty(h)&&p!=null&&!r.hasOwnProperty(h))switch(h){case`selected`:e.selected=!1;break;default:Id(e,t,h,null,r,p)}for(l in r)if(p=r[l],m=n[l],r.hasOwnProperty(l)&&p!==m&&(p!=null||m!=null))switch(l){case`selected`:e.selected=p&&typeof p!=`function`&&typeof p!=`symbol`;break;default:Id(e,t,l,p,r,m)}return;case`img`:case`link`:case`area`:case`base`:case`br`:case`col`:case`embed`:case`hr`:case`keygen`:case`meta`:case`param`:case`source`:case`track`:case`wbr`:case`menuitem`:for(var g in n)p=n[g],n.hasOwnProperty(g)&&p!=null&&!r.hasOwnProperty(g)&&Id(e,t,g,null,r,p);for(u in r)if(p=r[u],m=n[u],r.hasOwnProperty(u)&&p!==m&&(p!=null||m!=null))switch(u){case`children`:case`dangerouslySetInnerHTML`:if(p!=null)throw Error(s(137,t));break;default:Id(e,t,u,p,r,m)}return;default:if(an(t)){for(var _ in n)p=n[_],n.hasOwnProperty(_)&&p!==void 0&&!r.hasOwnProperty(_)&&Ld(e,t,_,void 0,r,p);for(d in r)p=r[d],m=n[d],!r.hasOwnProperty(d)||p===m||p===void 0&&m===void 0||Ld(e,t,d,p,r,m);return}}for(var v in n)p=n[v],n.hasOwnProperty(v)&&p!=null&&!r.hasOwnProperty(v)&&Id(e,t,v,null,r,p);for(f in r)p=r[f],m=n[f],!r.hasOwnProperty(f)||p===m||p==null&&m==null||Id(e,t,f,p,r,m)}function Bd(e){switch(e){case`css`:case`script`:case`font`:case`img`:case`image`:case`input`:case`link`:return!0;default:return!1}}function Vd(){if(typeof performance.getEntriesByType==`function`){for(var e=0,t=0,n=performance.getEntriesByType(`resource`),r=0;r<n.length;r++){var i=n[r],a=i.transferSize,o=i.initiatorType,s=i.duration;if(a&&s&&Bd(o)){for(o=0,s=i.responseEnd,r+=1;r<n.length;r++){var c=n[r],l=c.startTime;if(l>s)break;var u=c.transferSize,d=c.initiatorType;u&&Bd(d)&&(c=c.responseEnd,o+=u*(c<s?1:(s-l)/(c-l)))}if(--r,t+=8*(a+o)/(i.duration/1e3),e++,10<e)break}}if(0<e)return t/e/1e6}return navigator.connection&&(e=navigator.connection.downlink,typeof e==`number`)?e:5}var X=null,Hd=null;function Ud(e){return e.nodeType===9?e:e.ownerDocument}function Wd(e){switch(e){case`http://www.w3.org/2000/svg`:return 1;case`http://www.w3.org/1998/Math/MathML`:return 2;default:return 0}}function Gd(e,t){if(e===0)switch(t){case`svg`:return 1;case`math`:return 2;default:return 0}return e===1&&t===`foreignObject`?0:e}function Kd(e,t){return e===`textarea`||e===`noscript`||typeof t.children==`string`||typeof t.children==`number`||typeof t.children==`bigint`||typeof t.dangerouslySetInnerHTML==`object`&&t.dangerouslySetInnerHTML!==null&&t.dangerouslySetInnerHTML.__html!=null}var qd=null;function Jd(){var e=window.event;return e&&e.type===`popstate`?e===qd?!1:(qd=e,!0):(qd=null,!1)}var Yd=typeof setTimeout==`function`?setTimeout:void 0,Xd=typeof clearTimeout==`function`?clearTimeout:void 0,Zd=typeof Promise==`function`?Promise:void 0,Qd=typeof queueMicrotask==`function`?queueMicrotask:Zd===void 0?Yd:function(e){return Zd.resolve(null).then(e).catch($d)};function $d(e){setTimeout(function(){throw e})}function ef(e){return e===`head`}function tf(e,t){var n=t,r=0;do{var i=n.nextSibling;if(e.removeChild(n),i&&i.nodeType===8)if(n=i.data,n===`/$`||n===`/&`){if(r===0){e.removeChild(i),Fp(t);return}r--}else if(n===`$`||n===`$?`||n===`$~`||n===`$!`||n===`&`)r++;else if(n===`html`)gf(e.ownerDocument.documentElement);else if(n===`head`){n=e.ownerDocument.head,gf(n);for(var a=n.firstChild;a;){var o=a.nextSibling,s=a.nodeName;a[Ct]||s===`SCRIPT`||s===`STYLE`||s===`LINK`&&a.rel.toLowerCase()===`stylesheet`||n.removeChild(a),a=o}}else n===`body`&&gf(e.ownerDocument.body);n=i}while(n);Fp(t)}function nf(e,t){var n=e;e=0;do{var r=n.nextSibling;if(n.nodeType===1?t?(n._stashedDisplay=n.style.display,n.style.display=`none`):(n.style.display=n._stashedDisplay||``,n.getAttribute(`style`)===``&&n.removeAttribute(`style`)):n.nodeType===3&&(t?(n._stashedText=n.nodeValue,n.nodeValue=``):n.nodeValue=n._stashedText||``),r&&r.nodeType===8)if(n=r.data,n===`/$`){if(e===0)break;e--}else n!==`$`&&n!==`$?`&&n!==`$~`&&n!==`$!`||e++;n=r}while(n)}function rf(e){var t=e.firstChild;for(t&&t.nodeType===10&&(t=t.nextSibling);t;){var n=t;switch(t=t.nextSibling,n.nodeName){case`HTML`:case`HEAD`:case`BODY`:rf(n),wt(n);continue;case`SCRIPT`:case`STYLE`:continue;case`LINK`:if(n.rel.toLowerCase()===`stylesheet`)continue}e.removeChild(n)}}function af(e,t,n,r){for(;e.nodeType===1;){var i=n;if(e.nodeName.toLowerCase()!==t.toLowerCase()){if(!r&&(e.nodeName!==`INPUT`||e.type!==`hidden`))break}else if(!r)if(t===`input`&&e.type===`hidden`){var a=i.name==null?null:``+i.name;if(i.type===`hidden`&&e.getAttribute(`name`)===a)return e}else return e;else if(!e[Ct])switch(t){case`meta`:if(!e.hasAttribute(`itemprop`))break;return e;case`link`:if(a=e.getAttribute(`rel`),a===`stylesheet`&&e.hasAttribute(`data-precedence`)||a!==i.rel||e.getAttribute(`href`)!==(i.href==null||i.href===``?null:i.href)||e.getAttribute(`crossorigin`)!==(i.crossOrigin==null?null:i.crossOrigin)||e.getAttribute(`title`)!==(i.title==null?null:i.title))break;return e;case`style`:if(e.hasAttribute(`data-precedence`))break;return e;case`script`:if(a=e.getAttribute(`src`),(a!==(i.src==null?null:i.src)||e.getAttribute(`type`)!==(i.type==null?null:i.type)||e.getAttribute(`crossorigin`)!==(i.crossOrigin==null?null:i.crossOrigin))&&a&&e.hasAttribute(`async`)&&!e.hasAttribute(`itemprop`))break;return e;default:return e}if(e=df(e.nextSibling),e===null)break}return null}function of(e,t,n){if(t===``)return null;for(;e.nodeType!==3;)if((e.nodeType!==1||e.nodeName!==`INPUT`||e.type!==`hidden`)&&!n||(e=df(e.nextSibling),e===null))return null;return e}function sf(e,t){for(;e.nodeType!==8;)if((e.nodeType!==1||e.nodeName!==`INPUT`||e.type!==`hidden`)&&!t||(e=df(e.nextSibling),e===null))return null;return e}function cf(e){return e.data===`$?`||e.data===`$~`}function lf(e){return e.data===`$!`||e.data===`$?`&&e.ownerDocument.readyState!==`loading`}function uf(e,t){var n=e.ownerDocument;if(e.data===`$~`)e._reactRetry=t;else if(e.data!==`$?`||n.readyState!==`loading`)t();else{var r=function(){t(),n.removeEventListener(`DOMContentLoaded`,r)};n.addEventListener(`DOMContentLoaded`,r),e._reactRetry=r}}function df(e){for(;e!=null;e=e.nextSibling){var t=e.nodeType;if(t===1||t===3)break;if(t===8){if(t=e.data,t===`$`||t===`$!`||t===`$?`||t===`$~`||t===`&`||t===`F!`||t===`F`)break;if(t===`/$`||t===`/&`)return null}}return e}var ff=null;function pf(e){e=e.nextSibling;for(var t=0;e;){if(e.nodeType===8){var n=e.data;if(n===`/$`||n===`/&`){if(t===0)return df(e.nextSibling);t--}else n!==`$`&&n!==`$!`&&n!==`$?`&&n!==`$~`&&n!==`&`||t++}e=e.nextSibling}return null}function mf(e){e=e.previousSibling;for(var t=0;e;){if(e.nodeType===8){var n=e.data;if(n===`$`||n===`$!`||n===`$?`||n===`$~`||n===`&`){if(t===0)return e;t--}else n!==`/$`&&n!==`/&`||t++}e=e.previousSibling}return null}function hf(e,t,n){switch(t=Ud(n),e){case`html`:if(e=t.documentElement,!e)throw Error(s(452));return e;case`head`:if(e=t.head,!e)throw Error(s(453));return e;case`body`:if(e=t.body,!e)throw Error(s(454));return e;default:throw Error(s(451))}}function gf(e){for(var t=e.attributes;t.length;)e.removeAttributeNode(t[0]);wt(e)}var _f=new Map,vf=new Set;function yf(e){return typeof e.getRootNode==`function`?e.getRootNode():e.nodeType===9?e:e.ownerDocument}var bf=O.d;O.d={f:xf,r:Sf,D:Tf,C:Ef,L:Df,m:Of,X:Af,S:kf,M:jf};function xf(){var e=bf.f(),t=wu();return e||t}function Sf(e){var t=Et(e);t!==null&&t.tag===5&&t.type===`form`?Ms(t):bf.r(e)}var Cf=typeof document>`u`?null:document;function wf(e,t,n){var r=Cf;if(r&&typeof t==`string`&&t){var i=Jt(t);i=`link[rel="`+e+`"][href="`+i+`"]`,typeof n==`string`&&(i+=`[crossorigin="`+n+`"]`),vf.has(i)||(vf.add(i),e={rel:e,crossOrigin:n,href:t},r.querySelector(i)===null&&(t=r.createElement(`link`),Rd(t,`link`,e),kt(t),r.head.appendChild(t)))}}function Tf(e){bf.D(e),wf(`dns-prefetch`,e,null)}function Ef(e,t){bf.C(e,t),wf(`preconnect`,e,t)}function Df(e,t,n){bf.L(e,t,n);var r=Cf;if(r&&e&&t){var i=`link[rel="preload"][as="`+Jt(t)+`"]`;t===`image`&&n&&n.imageSrcSet?(i+=`[imagesrcset="`+Jt(n.imageSrcSet)+`"]`,typeof n.imageSizes==`string`&&(i+=`[imagesizes="`+Jt(n.imageSizes)+`"]`)):i+=`[href="`+Jt(e)+`"]`;var a=i;switch(t){case`style`:a=Z(e);break;case`script`:a=If(e)}_f.has(a)||(e=h({rel:`preload`,href:t===`image`&&n&&n.imageSrcSet?void 0:e,as:t},n),_f.set(a,e),r.querySelector(i)!==null||t===`style`&&r.querySelector(Nf(a))||t===`script`&&r.querySelector(Lf(a))||(t=r.createElement(`link`),Rd(t,`link`,e),kt(t),r.head.appendChild(t)))}}function Of(e,t){bf.m(e,t);var n=Cf;if(n&&e){var r=t&&typeof t.as==`string`?t.as:`script`,i=`link[rel="modulepreload"][as="`+Jt(r)+`"][href="`+Jt(e)+`"]`,a=i;switch(r){case`audioworklet`:case`paintworklet`:case`serviceworker`:case`sharedworker`:case`worker`:case`script`:a=If(e)}if(!_f.has(a)&&(e=h({rel:`modulepreload`,href:e},t),_f.set(a,e),n.querySelector(i)===null)){switch(r){case`audioworklet`:case`paintworklet`:case`serviceworker`:case`sharedworker`:case`worker`:case`script`:if(n.querySelector(Lf(a)))return}r=n.createElement(`link`),Rd(r,`link`,e),kt(r),n.head.appendChild(r)}}}function kf(e,t,n){bf.S(e,t,n);var r=Cf;if(r&&e){var i=Ot(r).hoistableStyles,a=Z(e);t||=`default`;var o=i.get(a);if(!o){var s={loading:0,preload:null};if(o=r.querySelector(Nf(a)))s.loading=5;else{e=h({rel:`stylesheet`,href:e,"data-precedence":t},n),(n=_f.get(a))&&Bf(e,n);var c=o=r.createElement(`link`);kt(c),Rd(c,`link`,e),c._p=new Promise(function(e,t){c.onload=e,c.onerror=t}),c.addEventListener(`load`,function(){s.loading|=1}),c.addEventListener(`error`,function(){s.loading|=2}),s.loading|=4,zf(o,t,r)}o={type:`stylesheet`,instance:o,count:1,state:s},i.set(a,o)}}}function Af(e,t){bf.X(e,t);var n=Cf;if(n&&e){var r=Ot(n).hoistableScripts,i=If(e),a=r.get(i);a||(a=n.querySelector(Lf(i)),a||(e=h({src:e,async:!0},t),(t=_f.get(i))&&Vf(e,t),a=n.createElement(`script`),kt(a),Rd(a,`link`,e),n.head.appendChild(a)),a={type:`script`,instance:a,count:1,state:null},r.set(i,a))}}function jf(e,t){bf.M(e,t);var n=Cf;if(n&&e){var r=Ot(n).hoistableScripts,i=If(e),a=r.get(i);a||(a=n.querySelector(Lf(i)),a||(e=h({src:e,async:!0,type:`module`},t),(t=_f.get(i))&&Vf(e,t),a=n.createElement(`script`),kt(a),Rd(a,`link`,e),n.head.appendChild(a)),a={type:`script`,instance:a,count:1,state:null},r.set(i,a))}}function Mf(e,t,n,r){var i=(i=_e.current)?yf(i):null;if(!i)throw Error(s(446));switch(e){case`meta`:case`title`:return null;case`style`:return typeof n.precedence==`string`&&typeof n.href==`string`?(t=Z(n.href),n=Ot(i).hoistableStyles,r=n.get(t),r||(r={type:`style`,instance:null,count:0,state:null},n.set(t,r)),r):{type:`void`,instance:null,count:0,state:null};case`link`:if(n.rel===`stylesheet`&&typeof n.href==`string`&&typeof n.precedence==`string`){e=Z(n.href);var a=Ot(i).hoistableStyles,o=a.get(e);if(o||(i=i.ownerDocument||i,o={type:`stylesheet`,instance:null,count:0,state:{loading:0,preload:null}},a.set(e,o),(a=i.querySelector(Nf(e)))&&!a._p&&(o.instance=a,o.state.loading=5),_f.has(e)||(n={rel:`preload`,as:`style`,href:n.href,crossOrigin:n.crossOrigin,integrity:n.integrity,media:n.media,hrefLang:n.hrefLang,referrerPolicy:n.referrerPolicy},_f.set(e,n),a||Ff(i,e,n,o.state))),t&&r===null)throw Error(s(528,``));return o}if(t&&r!==null)throw Error(s(529,``));return null;case`script`:return t=n.async,n=n.src,typeof n==`string`&&t&&typeof t!=`function`&&typeof t!=`symbol`?(t=If(n),n=Ot(i).hoistableScripts,r=n.get(t),r||(r={type:`script`,instance:null,count:0,state:null},n.set(t,r)),r):{type:`void`,instance:null,count:0,state:null};default:throw Error(s(444,e))}}function Z(e){return`href="`+Jt(e)+`"`}function Nf(e){return`link[rel="stylesheet"][`+e+`]`}function Pf(e){return h({},e,{"data-precedence":e.precedence,precedence:null})}function Ff(e,t,n,r){e.querySelector(`link[rel="preload"][as="style"][`+t+`]`)?r.loading=1:(t=e.createElement(`link`),r.preload=t,t.addEventListener(`load`,function(){return r.loading|=1}),t.addEventListener(`error`,function(){return r.loading|=2}),Rd(t,`link`,n),kt(t),e.head.appendChild(t))}function If(e){return`[src="`+Jt(e)+`"]`}function Lf(e){return`script[async]`+e}function Rf(e,t,n){if(t.count++,t.instance===null)switch(t.type){case`style`:var r=e.querySelector(`style[data-href~="`+Jt(n.href)+`"]`);if(r)return t.instance=r,kt(r),r;var i=h({},n,{"data-href":n.href,"data-precedence":n.precedence,href:null,precedence:null});return r=(e.ownerDocument||e).createElement(`style`),kt(r),Rd(r,`style`,i),zf(r,n.precedence,e),t.instance=r;case`stylesheet`:i=Z(n.href);var a=e.querySelector(Nf(i));if(a)return t.state.loading|=4,t.instance=a,kt(a),a;r=Pf(n),(i=_f.get(i))&&Bf(r,i),a=(e.ownerDocument||e).createElement(`link`),kt(a);var o=a;return o._p=new Promise(function(e,t){o.onload=e,o.onerror=t}),Rd(a,`link`,r),t.state.loading|=4,zf(a,n.precedence,e),t.instance=a;case`script`:return a=If(n.src),(i=e.querySelector(Lf(a)))?(t.instance=i,kt(i),i):(r=n,(i=_f.get(a))&&(r=h({},n),Vf(r,i)),e=e.ownerDocument||e,i=e.createElement(`script`),kt(i),Rd(i,`link`,r),e.head.appendChild(i),t.instance=i);case`void`:return null;default:throw Error(s(443,t.type))}else t.type===`stylesheet`&&!(t.state.loading&4)&&(r=t.instance,t.state.loading|=4,zf(r,n.precedence,e));return t.instance}function zf(e,t,n){for(var r=n.querySelectorAll(`link[rel="stylesheet"][data-precedence],style[data-precedence]`),i=r.length?r[r.length-1]:null,a=i,o=0;o<r.length;o++){var s=r[o];if(s.dataset.precedence===t)a=s;else if(a!==i)break}a?a.parentNode.insertBefore(e,a.nextSibling):(t=n.nodeType===9?n.head:n,t.insertBefore(e,t.firstChild))}function Bf(e,t){e.crossOrigin??=t.crossOrigin,e.referrerPolicy??=t.referrerPolicy,e.title??=t.title}function Vf(e,t){e.crossOrigin??=t.crossOrigin,e.referrerPolicy??=t.referrerPolicy,e.integrity??=t.integrity}var Hf=null;function Uf(e,t,n){if(Hf===null){var r=new Map,i=Hf=new Map;i.set(n,r)}else i=Hf,r=i.get(n),r||(r=new Map,i.set(n,r));if(r.has(e))return r;for(r.set(e,null),n=n.getElementsByTagName(e),i=0;i<n.length;i++){var a=n[i];if(!(a[Ct]||a[gt]||e===`link`&&a.getAttribute(`rel`)===`stylesheet`)&&a.namespaceURI!==`http://www.w3.org/2000/svg`){var o=a.getAttribute(t)||``;o=e+o;var s=r.get(o);s?s.push(a):r.set(o,[a])}}return r}function Wf(e,t,n){e=e.ownerDocument||e,e.head.insertBefore(n,t===`title`?e.querySelector(`head > title`):null)}function Gf(e,t,n){if(n===1||t.itemProp!=null)return!1;switch(e){case`meta`:case`title`:return!0;case`style`:if(typeof t.precedence!=`string`||typeof t.href!=`string`||t.href===``)break;return!0;case`link`:if(typeof t.rel!=`string`||typeof t.href!=`string`||t.href===``||t.onLoad||t.onError)break;switch(t.rel){case`stylesheet`:return e=t.disabled,typeof t.precedence==`string`&&e==null;default:return!0}case`script`:if(t.async&&typeof t.async!=`function`&&typeof t.async!=`symbol`&&!t.onLoad&&!t.onError&&t.src&&typeof t.src==`string`)return!0}return!1}function Kf(e){return!(e.type===`stylesheet`&&!(e.state.loading&3))}function qf(e,t,n,r){if(n.type===`stylesheet`&&(typeof r.media!=`string`||!1!==matchMedia(r.media).matches)&&!(n.state.loading&4)){if(n.instance===null){var i=Z(r.href),a=t.querySelector(Nf(i));if(a){t=a._p,typeof t==`object`&&t&&typeof t.then==`function`&&(e.count++,e=Xf.bind(e),t.then(e,e)),n.state.loading|=4,n.instance=a,kt(a);return}a=t.ownerDocument||t,r=Pf(r),(i=_f.get(i))&&Bf(r,i),a=a.createElement(`link`),kt(a);var o=a;o._p=new Promise(function(e,t){o.onload=e,o.onerror=t}),Rd(a,`link`,r),n.instance=a}e.stylesheets===null&&(e.stylesheets=new Map),e.stylesheets.set(n,t),(t=n.state.preload)&&!(n.state.loading&3)&&(e.count++,n=Xf.bind(e),t.addEventListener(`load`,n),t.addEventListener(`error`,n))}}var Jf=0;function Yf(e,t){return e.stylesheets&&e.count===0&&Qf(e,e.stylesheets),0<e.count||0<e.imgCount?function(n){var r=setTimeout(function(){if(e.stylesheets&&Qf(e,e.stylesheets),e.unsuspend){var t=e.unsuspend;e.unsuspend=null,t()}},6e4+t);0<e.imgBytes&&Jf===0&&(Jf=62500*Vd());var i=setTimeout(function(){if(e.waitingForImages=!1,e.count===0&&(e.stylesheets&&Qf(e,e.stylesheets),e.unsuspend)){var t=e.unsuspend;e.unsuspend=null,t()}},(e.imgBytes>Jf?50:800)+t);return e.unsuspend=n,function(){e.unsuspend=null,clearTimeout(r),clearTimeout(i)}}:null}function Xf(){if(this.count--,this.count===0&&(this.imgCount===0||!this.waitingForImages)){if(this.stylesheets)Qf(this,this.stylesheets);else if(this.unsuspend){var e=this.unsuspend;this.unsuspend=null,e()}}}var Zf=null;function Qf(e,t){e.stylesheets=null,e.unsuspend!==null&&(e.count++,Zf=new Map,t.forEach($f,e),Zf=null,Xf.call(e))}function $f(e,t){if(!(t.state.loading&4)){var n=Zf.get(e);if(n)var r=n.get(null);else{n=new Map,Zf.set(e,n);for(var i=e.querySelectorAll(`link[data-precedence],style[data-precedence]`),a=0;a<i.length;a++){var o=i[a];(o.nodeName===`LINK`||o.getAttribute(`media`)!==`not all`)&&(n.set(o.dataset.precedence,o),r=o)}r&&n.set(null,r)}i=t.instance,o=i.getAttribute(`data-precedence`),a=n.get(o)||r,a===r&&n.set(null,i),n.set(o,i),this.count++,r=Xf.bind(this),i.addEventListener(`load`,r),i.addEventListener(`error`,r),a?a.parentNode.insertBefore(i,a.nextSibling):(e=e.nodeType===9?e.head:e,e.insertBefore(i,e.firstChild)),t.state.loading|=4}}var ep={$$typeof:C,Provider:null,Consumer:null,_currentValue:le,_currentValue2:le,_threadCount:0};function tp(e,t,n,r,i,a,o,s,c){this.tag=1,this.containerInfo=e,this.pingCache=this.current=this.pendingChildren=null,this.timeoutHandle=-1,this.callbackNode=this.next=this.pendingContext=this.context=this.cancelPendingCommit=null,this.callbackPriority=0,this.expirationTimes=at(-1),this.entangledLanes=this.shellSuspendCounter=this.errorRecoveryDisabledLanes=this.expiredLanes=this.warmLanes=this.pingedLanes=this.suspendedLanes=this.pendingLanes=0,this.entanglements=at(0),this.hiddenUpdates=at(null),this.identifierPrefix=r,this.onUncaughtError=i,this.onCaughtError=a,this.onRecoverableError=o,this.pooledCache=null,this.pooledCacheLanes=0,this.formState=c,this.incompleteTransitions=new Map}function np(e,t,n,r,i,a,o,s,c,l,u,d){return e=new tp(e,t,n,o,c,l,u,d,s),t=1,!0===a&&(t|=24),a=hi(3,null,null,t),e.current=a,a.stateNode=e,t=ma(),t.refCount++,e.pooledCache=t,t.refCount++,a.memoizedState={element:r,isDehydrated:n,cache:t},qa(a),e}function rp(e){return e?(e=pi,e):pi}function ip(e,t,n,r,i,a){i=rp(i),r.context===null?r.context=i:r.pendingContext=i,r=Ya(t),r.payload={element:n},a=a===void 0?null:a,a!==null&&(r.callback=a),n=Xa(e,r,t),n!==null&&(K(n,e,t),Za(n,e,t))}function ap(e,t){if(e=e.memoizedState,e!==null&&e.dehydrated!==null){var n=e.retryLane;e.retryLane=n!==0&&n<t?n:t}}function op(e,t){ap(e,t),(e=e.alternate)&&ap(e,t)}function sp(e){if(e.tag===13||e.tag===31){var t=ui(e,67108864);t!==null&&K(t,e,67108864),op(e,67108864)}}function cp(e){if(e.tag===13||e.tag===31){var t=vu();t=dt(t);var n=ui(e,t);n!==null&&K(n,e,t),op(e,t)}}var lp=!0;function up(e,t,n,r){var i=D.T;D.T=null;var a=O.p;try{O.p=2,fp(e,t,n,r)}finally{O.p=a,D.T=i}}function dp(e,t,n,r){var i=D.T;D.T=null;var a=O.p;try{O.p=8,fp(e,t,n,r)}finally{O.p=a,D.T=i}}function fp(e,t,n,r){if(lp){var i=pp(r);if(i===null)Dd(e,t,r,mp,n),Tp(e,r);else if(Dp(i,e,t,n,r))r.stopPropagation();else if(Tp(e,r),t&4&&-1<wp.indexOf(e)){for(;i!==null;){var a=Et(i);if(a!==null)switch(a.tag){case 3:if(a=a.stateNode,a.current.memoizedState.isDehydrated){var o=et(a.pendingLanes);if(o!==0){var s=a;for(s.pendingLanes|=2,s.entangledLanes|=2;o;){var c=1<<31-qe(o);s.entanglements[1]|=c,o&=~c}od(a),!(B&6)&&(su=Fe()+500,sd(0,!1))}}break;case 31:case 13:s=ui(a,2),s!==null&&K(s,a,2),wu(),op(a,2)}if(a=pp(r),a===null&&Dd(e,t,r,mp,n),a===i)break;i=a}i!==null&&r.stopPropagation()}else Dd(e,t,r,null,n)}}function pp(e){return e=dn(e),hp(e)}var mp=null;function hp(e){if(mp=null,e=Tt(e),e!==null){var t=l(e);if(t===null)e=null;else{var n=t.tag;if(n===13){if(e=u(t),e!==null)return e;e=null}else if(n===31){if(e=d(t),e!==null)return e;e=null}else if(n===3){if(t.stateNode.current.memoizedState.isDehydrated)return t.tag===3?t.stateNode.containerInfo:null;e=null}else t!==e&&(e=null)}}return mp=e,null}function gp(e){switch(e){case`beforetoggle`:case`cancel`:case`click`:case`close`:case`contextmenu`:case`copy`:case`cut`:case`auxclick`:case`dblclick`:case`dragend`:case`dragstart`:case`drop`:case`focusin`:case`focusout`:case`input`:case`invalid`:case`keydown`:case`keypress`:case`keyup`:case`mousedown`:case`mouseup`:case`paste`:case`pause`:case`play`:case`pointercancel`:case`pointerdown`:case`pointerup`:case`ratechange`:case`reset`:case`resize`:case`seeked`:case`submit`:case`toggle`:case`touchcancel`:case`touchend`:case`touchstart`:case`volumechange`:case`change`:case`selectionchange`:case`textInput`:case`compositionstart`:case`compositionend`:case`compositionupdate`:case`beforeblur`:case`afterblur`:case`beforeinput`:case`blur`:case`fullscreenchange`:case`focus`:case`hashchange`:case`popstate`:case`select`:case`selectstart`:return 2;case`drag`:case`dragenter`:case`dragexit`:case`dragleave`:case`dragover`:case`mousemove`:case`mouseout`:case`mouseover`:case`pointermove`:case`pointerout`:case`pointerover`:case`scroll`:case`touchmove`:case`wheel`:case`mouseenter`:case`mouseleave`:case`pointerenter`:case`pointerleave`:return 8;case`message`:switch(Ie()){case Le:return 2;case Re:return 8;case ze:case Be:return 32;case Ve:return 268435456;default:return 32}default:return 32}}var _p=!1,vp=null,yp=null,bp=null,xp=new Map,Sp=new Map,Cp=[],wp=`mousedown mouseup touchcancel touchend touchstart auxclick dblclick pointercancel pointerdown pointerup dragend dragstart drop compositionend compositionstart keydown keypress keyup input textInput copy cut paste click change contextmenu reset`.split(` `);function Tp(e,t){switch(e){case`focusin`:case`focusout`:vp=null;break;case`dragenter`:case`dragleave`:yp=null;break;case`mouseover`:case`mouseout`:bp=null;break;case`pointerover`:case`pointerout`:xp.delete(t.pointerId);break;case`gotpointercapture`:case`lostpointercapture`:Sp.delete(t.pointerId)}}function Ep(e,t,n,r,i,a){return e===null||e.nativeEvent!==a?(e={blockedOn:t,domEventName:n,eventSystemFlags:r,nativeEvent:a,targetContainers:[i]},t!==null&&(t=Et(t),t!==null&&sp(t)),e):(e.eventSystemFlags|=r,t=e.targetContainers,i!==null&&t.indexOf(i)===-1&&t.push(i),e)}function Dp(e,t,n,r,i){switch(t){case`focusin`:return vp=Ep(vp,e,t,n,r,i),!0;case`dragenter`:return yp=Ep(yp,e,t,n,r,i),!0;case`mouseover`:return bp=Ep(bp,e,t,n,r,i),!0;case`pointerover`:var a=i.pointerId;return xp.set(a,Ep(xp.get(a)||null,e,t,n,r,i)),!0;case`gotpointercapture`:return a=i.pointerId,Sp.set(a,Ep(Sp.get(a)||null,e,t,n,r,i)),!0}return!1}function Op(e){var t=Tt(e.target);if(t!==null){var n=l(t);if(n!==null){if(t=n.tag,t===13){if(t=u(n),t!==null){e.blockedOn=t,mt(e.priority,function(){cp(n)});return}}else if(t===31){if(t=d(n),t!==null){e.blockedOn=t,mt(e.priority,function(){cp(n)});return}}else if(t===3&&n.stateNode.current.memoizedState.isDehydrated){e.blockedOn=n.tag===3?n.stateNode.containerInfo:null;return}}}e.blockedOn=null}function kp(e){if(e.blockedOn!==null)return!1;for(var t=e.targetContainers;0<t.length;){var n=pp(e.nativeEvent);if(n===null){n=e.nativeEvent;var r=new n.constructor(n.type,n);un=r,n.target.dispatchEvent(r),un=null}else return t=Et(n),t!==null&&sp(t),e.blockedOn=n,!1;t.shift()}return!0}function Ap(e,t,n){kp(e)&&n.delete(t)}function jp(){_p=!1,vp!==null&&kp(vp)&&(vp=null),yp!==null&&kp(yp)&&(yp=null),bp!==null&&kp(bp)&&(bp=null),xp.forEach(Ap),Sp.forEach(Ap)}function Mp(e,n){e.blockedOn===n&&(e.blockedOn=null,_p||(_p=!0,t.unstable_scheduleCallback(t.unstable_NormalPriority,jp)))}var Np=null;function Pp(e){Np!==e&&(Np=e,t.unstable_scheduleCallback(t.unstable_NormalPriority,function(){Np===e&&(Np=null);for(var t=0;t<e.length;t+=3){var n=e[t],r=e[t+1],i=e[t+2];if(typeof r!=`function`){if(hp(r||n)===null)continue;break}var a=Et(n);a!==null&&(e.splice(t,3),t-=3,As(a,{pending:!0,data:i,method:n.method,action:r},r,i))}}))}function Fp(e){function t(t){return Mp(t,e)}vp!==null&&Mp(vp,e),yp!==null&&Mp(yp,e),bp!==null&&Mp(bp,e),xp.forEach(t),Sp.forEach(t);for(var n=0;n<Cp.length;n++){var r=Cp[n];r.blockedOn===e&&(r.blockedOn=null)}for(;0<Cp.length&&(n=Cp[0],n.blockedOn===null);)Op(n),n.blockedOn===null&&Cp.shift();if(n=(e.ownerDocument||e).$$reactFormReplay,n!=null)for(r=0;r<n.length;r+=3){var i=n[r],a=n[r+1],o=i[_t]||null;if(typeof a==`function`)o||Pp(n);else if(o){var s=null;if(a&&a.hasAttribute(`formAction`)){if(i=a,o=a[_t]||null)s=o.formAction;else if(hp(i)!==null)continue}else s=o.action;typeof s==`function`?n[r+1]=s:(n.splice(r,3),r-=3),Pp(n)}}}function Ip(){function e(e){e.canIntercept&&e.info===`react-transition`&&e.intercept({handler:function(){return new Promise(function(e){return i=e})},focusReset:`manual`,scroll:`manual`})}function t(){i!==null&&(i(),i=null),r||setTimeout(n,20)}function n(){if(!r&&!navigation.transition){var e=navigation.currentEntry;e&&e.url!=null&&navigation.navigate(e.url,{state:e.getState(),info:`react-transition`,history:`replace`})}}if(typeof navigation==`object`){var r=!1,i=null;return navigation.addEventListener(`navigate`,e),navigation.addEventListener(`navigatesuccess`,t),navigation.addEventListener(`navigateerror`,t),setTimeout(n,100),function(){r=!0,navigation.removeEventListener(`navigate`,e),navigation.removeEventListener(`navigatesuccess`,t),navigation.removeEventListener(`navigateerror`,t),i!==null&&(i(),i=null)}}}function Lp(e){this._internalRoot=e}Rp.prototype.render=Lp.prototype.render=function(e){var t=this._internalRoot;if(t===null)throw Error(s(409));var n=t.current;ip(n,vu(),e,t,null,null)},Rp.prototype.unmount=Lp.prototype.unmount=function(){var e=this._internalRoot;if(e!==null){this._internalRoot=null;var t=e.containerInfo;ip(e.current,2,null,e,null,null),wu(),t[vt]=null}};function Rp(e){this._internalRoot=e}Rp.prototype.unstable_scheduleHydration=function(e){if(e){var t=pt();e={blockedOn:null,target:e,priority:t};for(var n=0;n<Cp.length&&t!==0&&t<Cp[n].priority;n++);Cp.splice(n,0,e),n===0&&Op(e)}};var zp=r.version;if(zp!==`19.2.7`)throw Error(s(527,zp,`19.2.7`));O.findDOMNode=function(e){var t=e._reactInternals;if(t===void 0)throw typeof e.render==`function`?Error(s(188)):(e=Object.keys(e).join(`,`),Error(s(268,e)));return e=p(t),e=e===null?null:m(e),e=e===null?null:e.stateNode,e};var Bp={bundleType:0,version:`19.2.7`,rendererPackageName:`react-dom`,currentDispatcherRef:D,reconcilerVersion:`19.2.7`};if(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__<`u`){var Vp=__REACT_DEVTOOLS_GLOBAL_HOOK__;if(!Vp.isDisabled&&Vp.supportsFiber)try{We=Vp.inject(Bp),Ge=Vp}catch{}}e.createRoot=function(e,t){if(!c(e))throw Error(s(299));var n=!1,r=``,i=$s,a=ec,o=tc;return t!=null&&(!0===t.unstable_strictMode&&(n=!0),t.identifierPrefix!==void 0&&(r=t.identifierPrefix),t.onUncaughtError!==void 0&&(i=t.onUncaughtError),t.onCaughtError!==void 0&&(a=t.onCaughtError),t.onRecoverableError!==void 0&&(o=t.onRecoverableError)),t=np(e,1,!1,null,null,n,r,null,i,a,o,Ip),e[vt]=t.current,Td(e),new Lp(t)}})),c=e(((e,t)=>{function n(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>`u`||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!=`function`))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(n)}catch(e){console.error(e)}}n(),t.exports=s()})),l=n(),u=c(),d=[{kind:`linear`,label:`Linear`,summary:`Passes the signal through unchanged; useful for regression outputs.`},{kind:`relu`,label:`ReLU`,summary:`Clips negative values to zero and keeps positive values, creating sparse activations.`},{kind:`leakyRelu`,label:`Leaky ReLU`,summary:`Keeps a small negative slope so negative inputs do not become completely silent.`},{kind:`sigmoid`,label:`Sigmoid`,summary:`Squashes values into 0 to 1, which is useful for probability-style outputs.`},{kind:`tanh`,label:`Tanh`,summary:`Squashes values into -1 to 1 and stays centered around zero.`},{kind:`softplus`,label:`Softplus`,summary:`A smooth ReLU-like curve that never has a sharp corner.`}];function f(e,t){switch(t){case`linear`:return e;case`relu`:return Math.max(0,e);case`leakyRelu`:return e>=0?e:e*.1;case`sigmoid`:return 1/(1+Math.exp(-e));case`tanh`:return Math.tanh(e);case`softplus`:return Math.log1p(Math.exp(-Math.abs(e)))+Math.max(e,0)}}function p(e){return d.find(t=>t.kind===e)??d[0]}var m=[{id:`red`,label:`red`,embedding:[1,0]},{id:`blue`,label:`blue`,embedding:[0,1]},{id:`purple`,label:`purple`,embedding:[1,1]}],h=[[1,0],[0,1]],g=[[1,1],[-1,1]],_=[[2,0],[0,1]];function v(e){return Math.abs(e)<1e-12?0:e}function y(e,t){if(e.length===0||t.length!==e.length||t.some(e=>e.length!==t[0].length)||![...e,...t.flat()].every(Number.isFinite))throw Error(`NN12 V1 needs finite row vectors and compatible rectangular matrices.`);return t[0].map((n,r)=>v(e.reduce((e,n,i)=>e+n*t[i][r],0)))}function b(e=m,t=h,n=g,r=_){if(e.length!==3||new Set(e.map(e=>e.id)).size!==e.length||e.some(e=>e.label.length===0||e.embedding.length!==2)||[t,n,r].some(e=>e.length!==2||e.some(e=>e.length!==2)))throw Error(`NN12 V1 needs three unique two-number tokens and three 2 x 2 matrices.`);let i=e.map(e=>({id:e.id,label:e.label,embedding:[...e.embedding],query:y(e.embedding,t),key:y(e.embedding,n),value:y(e.embedding,r)})),a=i[0].key.length,o=Math.sqrt(a),s=i.flatMap(e=>i.map(t=>{let n=e.query.map((e,n)=>v(e*t.key[n])),r=v(n.reduce((e,t)=>e+t,0));return{queryId:e.id,keyId:t.id,products:n,rawScore:r,scaledScore:v(r/o)}}));return{projections:i,dotProducts:s,rawScoreMatrix:i.map(e=>i.map(t=>s.find(n=>n.queryId===e.id&&n.keyId===t.id).rawScore)),scaledScoreMatrix:i.map(e=>i.map(t=>s.find(n=>n.queryId===e.id&&n.keyId===t.id).scaledScore)),scaleDivisor:o}}function x(e,t,n){let r=e.dotProducts.find(e=>e.queryId===t&&e.keyId===n);if(r===void 0)throw Error(`Unknown attention cell ${t} -> ${n}.`);return r}var S=b(),C=S.projections.map(e=>e.id),w=S.scaledScoreMatrix,ee=S.projections.map(e=>e.value);function te(e){return Math.abs(e)<1e-12?0:e}function T(e,t,n){return e.length===t&&e.every(e=>e.length===n&&e.every(Number.isFinite))}function ne(e=!0,t=w,n=ee,r=C){if(r.length!==3||new Set(r).size!==r.length||!T(t,3,3)||!T(n,3,2))throw Error(`NN13 V1 needs three token IDs, a finite 3 x 3 score matrix, and finite 3 x 2 values.`);let i=t.map((t,i)=>{let a=t.map((t,n)=>!e||n<=i),o=t.map((e,t)=>a[t]?e:null),s=Math.max(...o.filter(e=>e!==null)),c=o.map(e=>e===null?null:te(e-s)),l=c.map(e=>e===null?0:Math.exp(e)),u=l.reduce((e,t)=>e+t,0),d=l.map(e=>te(e/u)),f=n.map((e,t)=>e.map(e=>te(d[t]*e))),p=n[0].map((e,t)=>te(f.reduce((e,n)=>e+n[t],0)));return{queryId:r[i],allowed:a,scaledScores:[...t],maskedScores:o,rowMax:s,shiftedScores:c,exponentials:l,denominator:u,weights:d,values:n.map(e=>[...e]),valueContributions:f,context:p}});return{causal:e,tokenIds:[...r],rows:i,weightMatrix:i.map(e=>e.weights),contextMatrix:i.map(e=>e.context)}}function re(e,t){let n=e.rows.find(e=>e.queryId===t);if(n===void 0)throw Error(`Unknown attention softmax query ${t}.`);return n}var ie=e((e=>{var t=Symbol.for(`react.transitional.element`),n=Symbol.for(`react.fragment`);function r(e,n,r){var i=null;if(r!==void 0&&(i=``+r),n.key!==void 0&&(i=``+n.key),`key`in n)for(var a in r={},n)a!==`key`&&(r[a]=n[a]);else r=n;return n=r.ref,{$$typeof:t,type:e,key:i,ref:n===void 0?null:n,props:r}}e.Fragment=n,e.jsx=r,e.jsxs=r})),E=e(((e,t)=>{t.exports=ie()}))();function ae(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function oe(e){return`[${e.map(ae).join(`, `)}]`}function se(e){return`[${e.map(e=>e===null?`blocked`:ae(e)).join(`, `)}]`}function ce({onShowMultiHead:e,onShowScores:t}){let[n,r]=(0,l.useState)(!0),[i,a]=(0,l.useState)(`blue`),o=(0,l.useMemo)(()=>ne(n),[n]),s=re(o,i);return(0,E.jsxs)(`main`,{className:`workspace workspace--attention-softmax`,children:[(0,E.jsxs)(`section`,{className:`attention-softmax-stage`,"aria-label":`Causal attention weight trace`,children:[(0,E.jsxs)(`div`,{className:`attention-softmax-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN13 · normalize without looking ahead`}),(0,E.jsx)(`h2`,{children:`Causal-softmax mixer`}),(0,E.jsx)(`p`,{children:`Mask future keys, normalize one query row into weights, then follow each weight into the value vector it scales.`})]}),(0,E.jsx)(`div`,{className:`attention-softmax-chip`,children:n?`causal decoder`:`full context`})]}),(0,E.jsxs)(`section`,{className:`attention-weight-panel`,"aria-label":`Attention weight matrix`,children:[(0,E.jsxs)(`div`,{className:`attention-softmax-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Rows normalize independently`}),(0,E.jsxs)(`h2`,{children:[n?`Causal`:`Unmasked`,` attention weights`]})]}),(0,E.jsx)(`code`,{children:`each row sums to 1`})]}),(0,E.jsxs)(`div`,{className:`attention-weight-grid`,role:`grid`,"aria-label":`${n?`Causal`:`Unmasked`} attention weight matrix`,children:[(0,E.jsx)(`span`,{className:`attention-grid-corner`,children:`q \\ k`}),o.tokenIds.map(e=>(0,E.jsxs)(`span`,{className:`attention-grid-label`,children:[e,` k`]},`weight-key-${e}`)),o.rows.flatMap(e=>[(0,E.jsxs)(`button`,{"aria-label":`Select ${e.queryId} query row`,"aria-pressed":i===e.queryId,className:i===e.queryId?`attention-weight-row-button attention-weight-row-button--active`:`attention-weight-row-button`,type:`button`,onClick:()=>a(e.queryId),children:[e.queryId,` q`]},`weight-query-${e.queryId}`),...e.weights.map((t,n)=>{let r=!e.allowed[n];return(0,E.jsxs)(`div`,{"aria-label":`${e.queryId} query to ${o.tokenIds[n]} key: ${r?`blocked`:ae(t)}`,className:r?`attention-weight-cell attention-weight-cell--blocked`:i===e.queryId?`attention-weight-cell attention-weight-cell--selected-row`:`attention-weight-cell`,role:`gridcell`,children:[(0,E.jsx)(`strong`,{children:r?`blocked`:ae(t)}),(0,E.jsx)(`span`,{"aria-hidden":`true`,style:{width:`${Math.max(t*100,0)}%`}})]},`${e.queryId}-${o.tokenIds[n]}`)})])]})]}),(0,E.jsxs)(`section`,{className:`attention-normalize-panel`,"aria-label":`Selected softmax row trace`,children:[(0,E.jsxs)(`div`,{className:`attention-softmax-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected · `,s.queryId,` query`]}),(0,E.jsx)(`h2`,{children:`Score → mask → stable exponentials → weights`})]}),(0,E.jsxs)(`code`,{children:[`max = `,ae(s.rowMax)]})]}),(0,E.jsxs)(`div`,{className:`attention-normalize-flow`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`scaled scores`}),(0,E.jsx)(`code`,{children:oe(s.scaledScores)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`after mask`}),(0,E.jsx)(`code`,{children:se(s.maskedScores)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`subtract max, exp`}),(0,E.jsx)(`code`,{children:oe(s.exponentials)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,E.jsxs)(`div`,{className:`attention-normalize-flow__result`,children:[(0,E.jsxs)(`small`,{children:[`divide by `,ae(s.denominator)]}),(0,E.jsx)(`code`,{children:oe(s.weights)})]})]})]}),(0,E.jsxs)(`section`,{className:`attention-value-mix-panel`,"aria-label":`Selected weighted value mix`,children:[(0,E.jsxs)(`div`,{className:`attention-softmax-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Weights finally meet values`}),(0,E.jsxs)(`h2`,{children:[`Build the `,s.queryId,` context`]})]}),(0,E.jsxs)(`div`,{className:`attention-context-result`,children:[(0,E.jsx)(`small`,{children:`context`}),(0,E.jsx)(`strong`,{children:oe(s.context)})]})]}),(0,E.jsx)(`div`,{className:`attention-value-lanes`,children:o.tokenIds.map((e,t)=>(0,E.jsxs)(`div`,{className:s.allowed[t]?`attention-value-lane`:`attention-value-lane attention-value-lane--blocked`,children:[(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`attention-token-dot attention-token-dot--${e}`}),e,` value`]}),(0,E.jsxs)(`code`,{children:[ae(s.weights[t]),` × `,oe(s.values[t])]}),(0,E.jsxs)(`strong`,{children:[`= `,oe(s.valueContributions[t])]})]},e))})]})]}),(0,E.jsxs)(`aside`,{className:`attention-softmax-controls`,"aria-label":`Causal attention controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One information boundary`}),(0,E.jsx)(`h2`,{children:`Mask controls`}),(0,E.jsx)(`p`,{children:`Select a whole query row. Softmax belongs to that row, not to one score cell.`}),(0,E.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:t,children:`Return to Q/K/V scores`}),(0,E.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Open multi-head add and norm`}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:n,onChange:e=>r(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Block future keys`}),(0,E.jsx)(`small`,{children:`Allow column j only when j ≤ query row i.`})]})]}),(0,E.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Query row selection`,children:o.tokenIds.map(e=>(0,E.jsx)(`button`,{"aria-pressed":i===e,type:`button`,onClick:()=>a(e),children:e},e))}),(0,E.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected context`}),(0,E.jsx)(`strong`,{children:oe(s.context)}),(0,E.jsxs)(`span`,{children:[s.queryId,` reads `,s.allowed.filter(Boolean).length,` value`,s.allowed.filter(Boolean).length===1?``:`s`,`.`]})]}),(0,E.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,E.jsx)(`span`,{children:`Why subtract the maximum?`}),(0,E.jsx)(`p`,{children:`It keeps exponentials finite without changing their normalized proportions. The maximum shifted score is always zero.`})]}),(0,E.jsxs)(`div`,{className:`attention-next-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Multiple heads repeat this calculation with different projections, then concatenate their context vectors.`})]})]})]})}var D=[`red`,`blue`,`purple`],O=[`red`,`blue`,`purple`],le=[[1,0],[0,1]],ue=[[1,0,-1],[0,1,-1]],de=[0,0,0],fe=.5;function pe(e){return Math.abs(e)<1e-12?0:e}function me(e,t,n){return e.length===t&&e.every(e=>e.length===n&&e.every(Number.isFinite))}function he(e,t,n,r){let i=D.map((t,r)=>e.map((e,t)=>pe(e*n[t][r]))),a=i.map((e,t)=>pe(e.reduce((e,t)=>e+t,0)+r[t])),o=Math.max(...a),s=a.map(e=>pe(e-o)),c=s.map(Math.exp),l=c.reduce((e,t)=>e+t,0),u=c.map(e=>e/l),d=u[t];return{logitProducts:i,logits:a,rowMax:o,shiftedLogits:s,exponentials:c,denominator:l,probabilities:u,targetProbability:d,loss:-Math.log(d)}}function ge(e,t,n,r){return e.reduce((e,i,a)=>e+he(i,t[a],n,r).loss,0)/e.length}function _e(e=fe,t=le,n=ue,r=de){if(!Number.isFinite(e)||e<=0||!me(t,2,2)||!me(n,2,3)||r.length!==3||!r.every(Number.isFinite))throw Error(`NN15 V1 needs two 2D decoder states, a 2 x 3 unembedding, three finite biases, and a positive learning rate.`);let i=O.slice(0,-1),a=O.slice(1),o=Array.from({length:2},()=>[0,0,0]),s=[0,0,0],c=t.map((e,c)=>{let l=i[c],u=a[c],d=D.indexOf(u),f=he(e,d,n,r),p=f.probabilities.map((e,n)=>(e-+(n===d))/t.length),m=e.map(e=>p.map(t=>pe(e*t)));for(let e=0;e<2;e+=1)for(let t=0;t<3;t+=1)o[e][t]+=m[e][t];for(let e=0;e<3;e+=1)s[e]+=p[e];let h=e.map((e,t)=>pe(p.reduce((e,r,i)=>e+r*n[t][i],0)));return{position:c,inputToken:l,targetToken:u,targetIndex:d,causalPrefix:O.slice(0,c+1),decoderState:[...e],...f,logitGradients:p,unembeddingGradientContribution:m,biasGradientContribution:[...p],stateGradient:h}}),l=n.map((t,n)=>t.map((t,r)=>t-e*o[n][r])),u=r.map((t,n)=>t-e*s[n]),d=1e-6,f=c.map(e=>e.targetIndex),p=Array.from({length:2},()=>[0,0,0]);for(let e=0;e<2;e+=1)for(let i=0;i<3;i+=1){let a=n.map(e=>[...e]),o=n.map(e=>[...e]);a[e][i]+=d,o[e][i]-=d,p[e][i]=(ge(t,f,a,r)-ge(t,f,o,r))/(2*d)}let m=r.map((e,i)=>{let a=[...r],o=[...r];return a[i]+=d,o[i]-=d,(ge(t,f,n,a)-ge(t,f,n,o))/(2*d)}),h=[...o.flatMap((e,t)=>e.map((e,n)=>Math.abs(e-p[t][n]))),...s.map((e,t)=>Math.abs(e-m[t]))],g=c.map(e=>{let t=he(e.decoderState,e.targetIndex,l,u);return{position:e.position,logits:t.logits,probabilities:t.probabilities,targetProbability:t.targetProbability,loss:t.loss}});return{vocabulary:[...D],sequence:[...O],learningRate:e,rows:c,meanLoss:c.reduce((e,t)=>e+t.loss,0)/c.length,unembeddingGradient:o,biasGradient:s,gradientCheck:{epsilon:d,numericalUnembeddingGradient:p,numericalBiasGradient:m,maxAbsoluteError:Math.max(...h)},updatedUnembedding:l,updatedBias:u,postUpdateRows:g,postUpdateMeanLoss:g.reduce((e,t)=>e+t.loss,0)/g.length}}function ve(e,t){let n=e.rows[t];if(n===void 0)throw Error(`Unknown decoder training position ${t}.`);return n}function ye(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function be(e){return`[${e.map(ye).join(`, `)}]`}function xe(e){return e.map(be).join(`  `)}function Se({onShowMultiHead:e}){let t=(0,l.useMemo)(()=>_e(),[]),[n,r]=(0,l.useState)(1),[i,a]=(0,l.useState)(!1),o=ve(t,n),s=i?t.postUpdateRows[n]:o;return(0,E.jsxs)(`main`,{className:`workspace workspace--decoder`,children:[(0,E.jsxs)(`section`,{className:`decoder-stage`,"aria-label":`Tiny decoder language model training trace`,children:[(0,E.jsxs)(`div`,{className:`decoder-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN15 - a complete next-token learning step`}),(0,E.jsx)(`h2`,{children:`Tiny decoder training trace`}),(0,E.jsx)(`p`,{children:`Shift one sequence into two causal predictions, turn saved decoder states into vocabulary probabilities, then follow the shared error through cross-entropy and one loss-reducing SGD update.`})]}),(0,E.jsx)(`div`,{className:`decoder-chip`,children:`3-token vocabulary - 2 positions`})]}),(0,E.jsxs)(`section`,{className:`decoder-shift-panel`,"aria-label":`Causal next-token sequence shift`,children:[(0,E.jsxs)(`div`,{className:`decoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One sequence - shifted by one`}),(0,E.jsx)(`h2`,{children:`Prefixes predict what comes next`})]}),(0,E.jsx)(`code`,{children:`red blue purple`})]}),(0,E.jsx)(`div`,{className:`decoder-position-lanes`,children:t.rows.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Select position ${e.position}: ${e.causalPrefix.join(` `)} predicts ${e.targetToken}`,"aria-pressed":n===e.position,className:`decoder-position-button`,type:`button`,onClick:()=>r(e.position),children:[(0,E.jsxs)(`span`,{children:[`position `,e.position]}),(0,E.jsx)(`strong`,{children:e.causalPrefix.join(` `)}),(0,E.jsx)(`i`,{"aria-hidden":`true`,children:`->`}),(0,E.jsx)(`strong`,{children:e.targetToken}),(0,E.jsx)(`small`,{children:`future target stays outside the prefix`})]},e.position))})]}),(0,E.jsxs)(`section`,{className:`decoder-prediction-panel`,"aria-label":`Selected decoder prediction at position ${n}`,children:[(0,E.jsxs)(`div`,{className:`decoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected - position `,n]}),(0,E.jsx)(`h2`,{children:i?`Rerun the updated head`:`State to target surprise`})]}),(0,E.jsxs)(`div`,{className:`decoder-loss-badge`,children:[(0,E.jsx)(`small`,{children:`position loss`}),(0,E.jsx)(`strong`,{children:ye(s.loss)})]})]}),(0,E.jsxs)(`div`,{className:`decoder-forward-flow`,children:[(0,E.jsxs)(`div`,{className:`decoder-state-node`,children:[(0,E.jsx)(`small`,{children:`saved causal state`}),(0,E.jsxs)(`strong`,{children:[`h_`,o.inputToken]}),(0,E.jsx)(`code`,{children:be(o.decoderState)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`decoder-logit-node`,children:[(0,E.jsx)(`small`,{children:i?`updated logits`:`shared head logits`}),(0,E.jsx)(`code`,{children:be(s.logits)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`decoder-probability-node`,children:[(0,E.jsx)(`small`,{children:`stable softmax`}),(0,E.jsx)(`code`,{children:be(s.probabilities)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`decoder-target-node`,children:[(0,E.jsx)(`small`,{children:`target probability`}),(0,E.jsxs)(`strong`,{children:[`P(`,o.targetToken,`) = `,ye(s.targetProbability)]}),(0,E.jsxs)(`code`,{children:[`-ln(P) = `,ye(s.loss)]})]})]}),(0,E.jsx)(`div`,{className:`decoder-vocabulary-grid`,role:`list`,"aria-label":`Vocabulary probability distribution`,children:t.vocabulary.map((e,t)=>{let n=s.probabilities[t],r=t===o.targetIndex;return(0,E.jsxs)(`div`,{className:r?`decoder-vocabulary-row decoder-vocabulary-row--target`:`decoder-vocabulary-row`,role:`listitem`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`span`,{children:[e,r?` - target`:``]}),(0,E.jsx)(`strong`,{children:ye(n)})]}),(0,E.jsx)(`i`,{"aria-hidden":`true`,style:{width:`${n*100}%`}}),i?null:(0,E.jsxs)(`code`,{children:[ye(o.logitProducts[t][0]),` + `,ye(o.logitProducts[t][1]),` + bias = `,ye(o.logits[t])]})]},e)})}),i?null:(0,E.jsxs)(`div`,{className:`decoder-softmax-trace`,"aria-label":`Stable softmax arithmetic`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`row max`}),(0,E.jsx)(`code`,{children:ye(o.rowMax)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`shift logits`}),(0,E.jsx)(`code`,{children:be(o.shiftedLogits)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`exponentials`}),(0,E.jsx)(`code`,{children:be(o.exponentials)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`denominator`}),(0,E.jsx)(`code`,{children:ye(o.denominator)})]})]})]}),(0,E.jsxs)(`section`,{className:`decoder-gradient-panel`,"aria-label":`Decoder loss gradient trace`,children:[(0,E.jsxs)(`div`,{className:`decoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Probability minus target - divided by two`}),(0,E.jsx)(`h2`,{children:`Error flows back through the shared head`})]}),(0,E.jsx)(`code`,{children:`(p - one_hot) / positions`})]}),(0,E.jsxs)(`div`,{className:`decoder-gradient-flow`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`logit gradient`}),(0,E.jsx)(`code`,{children:be(o.logitGradients)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`this position's unembedding contribution`}),(0,E.jsx)(`code`,{children:xe(o.unembeddingGradientContribution)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias contribution`}),(0,E.jsx)(`code`,{children:be(o.biasGradientContribution)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`decoder-state-gradient`,children:[(0,E.jsx)(`small`,{children:`gradient entering decoder body`}),(0,E.jsx)(`code`,{children:be(o.stateGradient)})]})]})]}),(0,E.jsxs)(`section`,{className:`decoder-update-panel`,"aria-label":`Shared decoder head SGD update`,children:[(0,E.jsxs)(`div`,{className:`decoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Both positions reduce into one update`}),(0,E.jsx)(`h2`,{children:`Shared-head SGD checkpoint`})]}),(0,E.jsxs)(`code`,{children:[`parameter - `,t.learningRate,` x gradient`]})]}),(0,E.jsxs)(`div`,{className:`decoder-update-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`unembedding before`}),(0,E.jsx)(`code`,{children:xe(ue)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`reduced gradient`}),(0,E.jsx)(`code`,{children:xe(t.unembeddingGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`unembedding after`}),(0,E.jsx)(`code`,{children:xe(t.updatedUnembedding)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias before`}),(0,E.jsx)(`code`,{children:be(de)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias gradient`}),(0,E.jsx)(`code`,{children:be(t.biasGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias after`}),(0,E.jsx)(`code`,{children:be(t.updatedBias)})]})]}),(0,E.jsxs)(`div`,{className:`decoder-gradient-audit`,children:[(0,E.jsx)(`span`,{children:`Central finite-difference audit`}),(0,E.jsxs)(`code`,{children:[`epsilon = `,t.gradientCheck.epsilon]}),(0,E.jsxs)(`strong`,{children:[`max error `,t.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,E.jsxs)(`div`,{className:`decoder-loss-drop`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean loss before`}),(0,E.jsx)(`strong`,{children:ye(t.meanLoss)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean loss after one step`}),(0,E.jsx)(`strong`,{children:ye(t.postUpdateMeanLoss)})]}),(0,E.jsx)(`p`,{children:`Both target probabilities rise; the deterministic objective falls.`})]})]})]}),(0,E.jsxs)(`aside`,{className:`decoder-controls`,"aria-label":`Tiny decoder training controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Inspect one prediction`}),(0,E.jsx)(`h2`,{children:`Training controls`}),(0,E.jsx)(`p`,{children:`The causal prefixes and saved states do not change. The toggle swaps only the shared vocabulary head before and after its one SGD step.`}),(0,E.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Return to multi-head block`}),(0,E.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Decoder position selection`,children:t.rows.map(e=>(0,E.jsxs)(`button`,{"aria-pressed":n===e.position,type:`button`,onClick:()=>r(e.position),children:[`position `,e.position]},e.position))}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:i,onChange:e=>a(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Use updated vocabulary head`}),(0,E.jsx)(`small`,{children:`Rerun logits and loss after one SGD step.`})]})]}),(0,E.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected target`}),(0,E.jsx)(`strong`,{children:o.targetToken}),(0,E.jsxs)(`span`,{children:[o.causalPrefix.join(` `),` -> `,o.targetToken]})]}),(0,E.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,E.jsx)(`span`,{children:`Frozen on purpose`}),(0,E.jsx)(`p`,{children:`This first trace updates unembedding and bias. The state gradient is preserved for a later full-decoder autograd pass.`})]}),(0,E.jsxs)(`div`,{className:`attention-next-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Add token sampling and a generation trace, then continue the saved state gradients through every decoder-block parameter.`})]})]})]})}var Ce=[`red`,`blue`,`purple`],we=[[2,0],[0,1],[2,1]],Te=[{id:`horizontal`,queryProjection:[.5,0],keyProjection:[.5,0],valueProjection:[1,0]},{id:`vertical`,queryProjection:[0,1],keyProjection:[0,1],valueProjection:[0,1]}],Ee=[[1,0],[0,1]],De={epsilon:1e-5,gamma:[1,1],beta:[0,0]};function Oe(e){return Math.abs(e)<1e-12?0:e}function ke(e,t,n){return e.length===t&&e.every(e=>e.length===n&&e.every(Number.isFinite))}function Ae(e,t){return e.map((e,n)=>Oe(e*t[n]))}function je(e,t,n){let r=Ae(e[t],n.queryProjection),i=Oe(r.reduce((e,t)=>e+t,0)),a=e.map(e=>Ae(e,n.keyProjection)),o=a.map(e=>Oe(e.reduce((e,t)=>e+t,0))),s=e.map(e=>Ae(e,n.valueProjection)),c=s.map(e=>Oe(e.reduce((e,t)=>e+t,0))),l=o.map(e=>Oe(i*e/1)),u=l.map((e,n)=>n<=t),d=l.map((e,t)=>u[t]?e:null),f=Math.max(...d.filter(e=>e!==null)),p=d.map(e=>e===null?null:Oe(e-f)),m=p.map(e=>e===null?0:Math.exp(e)),h=m.reduce((e,t)=>e+t,0),g=m.map(e=>Oe(e/h)),_=g.map((e,t)=>Oe(e*c[t]));return{id:n.id,queryProducts:r,query:i,keyProducts:a,keys:o,valueProducts:s,values:c,scaleDivisor:1,scaledScores:l,allowed:u,maskedScores:d,rowMax:f,shiftedScores:p,exponentials:m,denominator:h,weights:g,valueContributions:_,context:Oe(_.reduce((e,t)=>e+t,0))}}function Me(e=!0,t=!0,n=we,r=Ce,i=Te,a=Ee,o=De.epsilon,s=De.gamma,c=De.beta){if(r.length!==3||new Set(r).size!==3||!ke(n,3,2)||i.length!==2||new Set(i.map(e=>e.id)).size!==2||i.some(e=>!ke([e.queryProjection,e.keyProjection,e.valueProjection],3,2))||!ke(a,2,2)||!Number.isFinite(o)||o<=0||!ke([s,c],2,2))throw Error(`NN14 V1 needs three 2D tokens, two scalar heads, a 2 x 2 output projection, and finite layer-norm parameters.`);let l=n.map((l,u)=>{let d=i.map(e=>je(n,u,e)),f=d.map(e=>e.context),p=a[0].map((e,t)=>f.map((e,n)=>Oe(e*a[n][t]))),m=p.map(e=>Oe(e.reduce((e,t)=>e+t,0))),h=m.map((t,n)=>Oe(t+(e?l[n]:0))),g=h.reduce((e,t)=>e+t,0)/2,_=h.map(e=>Oe(e-g)),v=_.map(e=>e*e),y=v.reduce((e,t)=>e+t,0)/2,b=Math.sqrt(y+o),x=_.map(e=>Oe(e/b)),S=x.map((e,t)=>Oe(e*s[t])),C=S.map((e,t)=>Oe(e+c[t]));return{tokenId:r[u],input:[...l],heads:d,concatenated:f,outputProjectionProducts:p,projectedAttention:m,residualSum:h,layerNorm:{mean:g,centered:_,squaredDeviations:v,variance:y,denominator:b,normalized:x,affineProducts:S,output:C},output:t?C:h}});return{includeResidual:e,applyLayerNorm:t,tokenIds:[...r],rows:l}}function Ne(e,t){let n=e.rows.find(e=>e.tokenId===t);if(n===void 0)throw Error(`Unknown multi-head attention token ${t}.`);return n}function Pe(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function Fe(e){return`[${e.map(Pe).join(`, `)}]`}function Ie(e){return e===`horizontal`?`Head A - horizontal`:`Head B - vertical`}function Le({onShowDecoder:e,onShowWeights:t}){let[n,r]=(0,l.useState)(`blue`),[i,a]=(0,l.useState)(!0),[o,s]=(0,l.useState)(!0),c=(0,l.useMemo)(()=>Me(i,o),[o,i]),u=Ne(c,n);return(0,E.jsxs)(`main`,{className:`workspace workspace--multi-head`,children:[(0,E.jsxs)(`section`,{className:`multi-head-stage`,"aria-label":`Multi-head attention block trace`,children:[(0,E.jsxs)(`div`,{className:`multi-head-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN14 - parallel views rejoin one stream`}),(0,E.jsx)(`h2`,{children:`Multi-head add-and-norm block`}),(0,E.jsx)(`p`,{children:`Run two causal heads on the same token, keep their weights separate, then follow concatenation, projection, residual, and layer normalization without skipping a boundary.`})]}),(0,E.jsx)(`div`,{className:`multi-head-chip`,children:`2 heads x 1 feature`})]}),(0,E.jsxs)(`section`,{className:`multi-head-panel`,"aria-label":`Two attention heads for ${n}`,children:[(0,E.jsxs)(`div`,{className:`multi-head-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected - `,n,` query`]}),(0,E.jsx)(`h2`,{children:`Same token, different learned views`})]}),(0,E.jsx)(`code`,{children:`each head softmaxes alone`})]}),(0,E.jsx)(`div`,{className:`multi-head-lanes`,children:u.heads.map(e=>(0,E.jsxs)(`article`,{className:`multi-head-lane multi-head-lane--${e.id}`,children:[(0,E.jsxs)(`div`,{className:`multi-head-lane__heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:Ie(e.id)}),(0,E.jsxs)(`strong`,{children:[`q = `,Pe(e.query)]})]}),(0,E.jsxs)(`code`,{children:[`context `,Pe(e.context)]})]}),(0,E.jsxs)(`div`,{className:`multi-head-score-row`,children:[(0,E.jsx)(`span`,{children:`scores`}),(0,E.jsx)(`code`,{children:Fe(e.scaledScores)})]}),(0,E.jsx)(`div`,{className:`multi-head-weight-row`,role:`list`,"aria-label":`${e.id} weights`,children:c.tokenIds.map((t,n)=>(0,E.jsxs)(`div`,{className:e.allowed[n]?`multi-head-weight`:`multi-head-weight multi-head-weight--blocked`,role:`listitem`,children:[(0,E.jsx)(`span`,{children:t}),(0,E.jsx)(`strong`,{children:e.allowed[n]?Pe(e.weights[n]):`blocked`}),(0,E.jsx)(`i`,{"aria-hidden":`true`,style:{width:`${e.weights[n]*100}%`}})]},t))}),(0,E.jsx)(`div`,{className:`multi-head-value-row`,children:c.tokenIds.map((t,n)=>(0,E.jsxs)(`code`,{children:[Pe(e.weights[n]),` x `,Pe(e.values[n]),` = `,Pe(e.valueContributions[n])]},t))})]},e.id))})]}),(0,E.jsxs)(`section`,{className:`multi-head-join-panel`,"aria-label":`Concatenate project and add residual trace`,children:[(0,E.jsxs)(`div`,{className:`multi-head-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Heads rejoin before the shortcut`}),(0,E.jsx)(`h2`,{children:`Concatenate - project - add`})]}),(0,E.jsx)(`code`,{children:`model width = 2`})]}),(0,E.jsxs)(`div`,{className:`multi-head-join-flow`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`head contexts`}),(0,E.jsx)(`code`,{children:Fe(u.heads.map(e=>e.context))})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`concatenate`}),(0,E.jsx)(`code`,{children:Fe(u.concatenated)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`identity W_o`}),(0,E.jsx)(`code`,{children:Fe(u.projectedAttention)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,E.jsxs)(`div`,{className:i?`multi-head-residual`:`multi-head-residual multi-head-residual--off`,children:[(0,E.jsx)(`small`,{children:i?`${n} residual`:`residual removed`}),(0,E.jsx)(`code`,{children:Fe(i?u.input:[0,0])})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,E.jsxs)(`div`,{className:`multi-head-join-result`,children:[(0,E.jsx)(`small`,{children:`add result`}),(0,E.jsx)(`code`,{children:Fe(u.residualSum)})]})]})]}),(0,E.jsxs)(`section`,{className:`multi-head-norm-panel`,"aria-label":`Layer normalization arithmetic`,children:[(0,E.jsxs)(`div`,{className:`multi-head-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One token - normalize across features`}),(0,E.jsx)(`h2`,{children:o?`Layer normalization`:`Layer normalization bypassed`})]}),(0,E.jsxs)(`div`,{className:`multi-head-output`,children:[(0,E.jsx)(`small`,{children:`block output`}),(0,E.jsx)(`strong`,{children:Fe(u.output)})]})]}),(0,E.jsxs)(`div`,{className:o?`multi-head-norm-flow`:`multi-head-norm-flow multi-head-norm-flow--off`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean`}),(0,E.jsx)(`code`,{children:Pe(u.layerNorm.mean)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`centered`}),(0,E.jsx)(`code`,{children:Fe(u.layerNorm.centered)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`squared deviations`}),(0,E.jsx)(`code`,{children:Fe(u.layerNorm.squaredDeviations)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`variance`}),(0,E.jsx)(`code`,{children:Pe(u.layerNorm.variance)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`sqrt(var + 0.00001)`}),(0,E.jsx)(`code`,{children:Pe(u.layerNorm.denominator)})]}),(0,E.jsxs)(`div`,{className:`multi-head-norm-result`,children:[(0,E.jsx)(`small`,{children:`gamma x normalized + beta`}),(0,E.jsx)(`code`,{children:Fe(u.layerNorm.output)})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`multi-head-controls`,"aria-label":`Multi-head attention controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Inspect one token row`}),(0,E.jsx)(`h2`,{children:`Block controls`}),(0,E.jsx)(`p`,{children:`Both heads stay visible so their different scores and value mixes can be compared on the same causal boundary.`}),(0,E.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:t,children:`Return to single-head weights`}),(0,E.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Open tiny decoder training`}),(0,E.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Multi-head token selection`,children:c.tokenIds.map(e=>(0,E.jsx)(`button`,{"aria-pressed":n===e,type:`button`,onClick:()=>r(e),children:e},e))}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:i,onChange:e=>a(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Add residual token`}),(0,E.jsx)(`small`,{children:`Keep the original embedding on a short route.`})]})]}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:o,onChange:e=>s(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Apply layer normalization`}),(0,E.jsx)(`small`,{children:`Use population variance across this token's features.`})]})]}),(0,E.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected block output`}),(0,E.jsx)(`strong`,{children:Fe(u.output)}),(0,E.jsxs)(`span`,{children:[n,` after both head paths rejoin.`]})]}),(0,E.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,E.jsx)(`span`,{children:`Why keep the heads separate?`}),(0,E.jsx)(`p`,{children:`A softmax row belongs to one head. Concatenation happens only after each head has produced its own context.`})]}),(0,E.jsxs)(`div`,{className:`attention-next-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`A decoder repeats this block across tokens and layers, then adds embeddings, a feed-forward path, logits, loss, and an optimizer.`})]})]})]})}function Re(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function ze(e){return`[${e.map(Re).join(`, `)}]`}function Be({onShowWeights:e}){let t=(0,l.useMemo)(()=>b(),[]),[n,r]=(0,l.useState)(`blue`),[i,a]=(0,l.useState)(`purple`),[o,s]=(0,l.useState)(!1),c=x(t,n,i),u=t.projections.find(e=>e.id===n),d=t.projections.find(e=>e.id===i),f=o?t.scaledScoreMatrix:t.rawScoreMatrix;return(0,E.jsxs)(`main`,{className:`workspace workspace--attention`,children:[(0,E.jsxs)(`section`,{className:`attention-stage`,"aria-label":`Three-token attention score trace`,children:[(0,E.jsxs)(`div`,{className:`attention-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN12 · attention foundations`}),(0,E.jsx)(`h2`,{children:`Query-key score microscope`}),(0,E.jsx)(`p`,{children:`Give every token three jobs, then open any score cell to see the two multiplications and one addition behind its match strength.`})]}),(0,E.jsx)(`div`,{className:`attention-sequence-chip`,children:`red · blue · purple`})]}),(0,E.jsxs)(`section`,{className:`attention-projection-panel`,"aria-label":`Token projections`,children:[(0,E.jsxs)(`div`,{className:`attention-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One token · three projections`}),(0,E.jsx)(`h2`,{children:`Ask, advertise, carry`})]}),(0,E.jsx)(`p`,{children:`Each row uses the same three learned matrices.`})]}),(0,E.jsxs)(`div`,{className:`attention-projection-table`,children:[(0,E.jsxs)(`div`,{className:`attention-projection-head`,"aria-hidden":`true`,children:[(0,E.jsx)(`span`,{children:`token x`}),(0,E.jsx)(`span`,{children:`query q`}),(0,E.jsx)(`span`,{children:`key k`}),(0,E.jsx)(`span`,{children:`value v`})]}),t.projections.map(e=>(0,E.jsxs)(`div`,{className:`attention-projection-row`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`i`,{className:`attention-token-dot attention-token-dot--${e.id}`}),(0,E.jsx)(`strong`,{children:e.label}),(0,E.jsx)(`code`,{children:ze(e.embedding)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`asks with`}),(0,E.jsx)(`code`,{children:ze(e.query)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`matches with`}),(0,E.jsx)(`code`,{children:ze(e.key)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`carries`}),(0,E.jsx)(`code`,{children:ze(e.value)})]})]},e.id))]})]}),(0,E.jsxs)(`section`,{className:`attention-score-panel`,"aria-label":`Query-key score matrix`,children:[(0,E.jsxs)(`div`,{className:`attention-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Rows ask · columns match`}),(0,E.jsxs)(`h2`,{children:[o?`Scaled`:`Raw`,` query-key scores`]})]}),(0,E.jsx)(`code`,{children:o?`QK^T / sqrt(2)`:`QK^T`})]}),(0,E.jsxs)(`div`,{className:`attention-score-layout`,children:[(0,E.jsxs)(`div`,{className:`attention-score-grid`,role:`grid`,"aria-label":`${o?`Scaled`:`Raw`} attention scores`,children:[(0,E.jsx)(`span`,{className:`attention-grid-corner`,children:`q \\ k`}),t.projections.map(e=>(0,E.jsxs)(`span`,{className:`attention-grid-label`,children:[e.label,` k`]},`key-${e.id}`)),t.projections.flatMap((e,o)=>[(0,E.jsxs)(`span`,{className:`attention-grid-label`,children:[e.label,` q`]},`query-${e.id}`),...t.projections.map((t,s)=>{let c=n===e.id&&i===t.id;return(0,E.jsx)(`button`,{"aria-label":`Select ${e.label} query and ${t.label} key`,"aria-selected":c,className:c?`attention-score-cell attention-score-cell--active`:`attention-score-cell`,role:`gridcell`,type:`button`,onClick:()=>{r(e.id),a(t.id)},children:Re(f[o][s])},`${e.id}-${t.id}`)})])]}),(0,E.jsxs)(`div`,{className:`attention-cell-trace`,"aria-label":`Selected dot-product arithmetic`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Selected cell`}),(0,E.jsxs)(`h3`,{children:[u.label,` asks · `,d.label,` matches`]})]}),(0,E.jsxs)(`div`,{className:`attention-vector-pair`,children:[(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`small`,{children:`query`}),(0,E.jsxs)(`code`,{children:[`q_`,u.id,` = `,ze(u.query)]})]}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`small`,{children:`key`}),(0,E.jsxs)(`code`,{children:[`k_`,d.id,` = `,ze(d.key)]})]})]}),(0,E.jsxs)(`div`,{className:`attention-dot-equation`,children:[(0,E.jsx)(`code`,{children:`${Re(u.query[0])} × ${Re(d.key[0])} + ${Re(u.query[1])} × ${Re(d.key[1])}`}),(0,E.jsxs)(`strong`,{children:[`= `,Re(c.rawScore)]})]}),(0,E.jsxs)(`div`,{className:`attention-products`,children:[`coordinate products `,ze(c.products)]}),o?(0,E.jsxs)(`div`,{className:`attention-scale-equation`,children:[Re(c.rawScore),` / sqrt(2) = `,(0,E.jsx)(`strong`,{children:Re(c.scaledScore)})]}):null]})]})]})]}),(0,E.jsxs)(`aside`,{className:`attention-controls`,"aria-label":`Attention score controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Keep the boundary honest`}),(0,E.jsx)(`h2`,{children:`Score controls`}),(0,E.jsx)(`p`,{children:`A score says how strongly one query matches one key. It does not yet say how much of a value to blend.`}),(0,E.jsx)(`button`,{className:`attention-back-button`,type:`button`,onClick:e,children:`Apply softmax and causal mask`}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:o,onChange:e=>s(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Scale by sqrt(key dimension)`}),(0,E.jsx)(`small`,{children:`Divide every raw score by sqrt(2).`})]})]}),(0,E.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected score`}),(0,E.jsx)(`strong`,{children:Re(o?c.scaledScore:c.rawScore)}),(0,E.jsxs)(`span`,{children:[u.label,` query → `,d.label,` key`]})]}),(0,E.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,E.jsx)(`span`,{children:`Value waiting downstream`}),(0,E.jsxs)(`code`,{children:[`v_`,d.id,` = `,ze(d.value)]}),(0,E.jsx)(`p`,{children:`This payload does not enter the score calculation.`})]}),(0,E.jsxs)(`div`,{className:`attention-next-note`,children:[(0,E.jsx)(`span`,{children:`What comes next?`}),(0,E.jsx)(`p`,{children:`Open the next view to turn each score row into weights and use those weights to blend the value vectors.`})]})]})]})}function Ve(){let[e,t]=(0,l.useState)(`scores`);return e===`weights`?(0,E.jsx)(ce,{onShowMultiHead:()=>t(`multi-head`),onShowScores:()=>t(`scores`)}):e===`multi-head`?(0,E.jsx)(Le,{onShowDecoder:()=>t(`decoder`),onShowWeights:()=>t(`weights`)}):e===`decoder`?(0,E.jsx)(Se,{onShowMultiHead:()=>t(`multi-head`)}):(0,E.jsx)(Be,{onShowWeights:()=>t(`weights`)})}function He(e){if(typeof e==`string`)return`string:${e}`;if(typeof e==`number`||typeof e==`bigint`||typeof e==`boolean`||typeof e==`symbol`||e==null)return`${typeof e}:${String(e)}`;try{return`json:${JSON.stringify(e)}`}catch{return`string:${String(e)}`}}function Ue(e,t){return He(e).localeCompare(He(t))}var We=class extends Error{node;constructor(e){super(`Node not found: ${String(e)}`),this.node=e,this.name=`NodeNotFoundError`}},Ge=class extends Error{edgeId;constructor(e){super(`Edge not found: ${e}`),this.edgeId=e,this.name=`EdgeNotFoundError`}},Ke=class extends Error{edgeId;constructor(e){super(`Edge ID already exists: ${e}`),this.edgeId=e,this.name=`DuplicateEdgeIdError`}},qe=class extends Error{constructor(e){super(e),this.name=`MultiDirectedGraphCycleError`}},Je=class{_allowSelfLoops;_nodes=new Set;_edges=new Map;_outgoing=new Map;_incoming=new Map;_graphProperties={};_nodeProperties=new Map;_edgeProperties=new Map;_nextEdgeId=0;constructor(e={}){this._allowSelfLoops=e.allowSelfLoops??!1}get allowSelfLoops(){return this._allowSelfLoops}get size(){return this._nodes.size}addNode(e,t={}){this._nodes.has(e)||(this._nodes.add(e),this._outgoing.set(e,new Set),this._incoming.set(e,new Set),this._nodeProperties.set(e,{})),Object.assign(this._nodeProperties.get(e),t)}removeNode(e){this.assertNode(e);let t=new Set([...this._outgoing.get(e),...this._incoming.get(e)]);for(let e of t)this.removeEdge(e);this._nodes.delete(e),this._outgoing.delete(e),this._incoming.delete(e),this._nodeProperties.delete(e)}hasNode(e){return this._nodes.has(e)}nodes(){return Array.from(this._nodes)}addEdge(e,t,n=1,r={},i){if(e===t&&!this._allowSelfLoops)throw Error(`Self-loops are not allowed: ${String(e)} -> ${String(t)}`);this.validateWeight(n);let a=i??this.allocateEdgeId();if(this._edges.has(a))throw new Ke(a);this.addNode(e),this.addNode(t);let o={id:a,from:e,to:t,weight:n};return this._edges.set(a,o),this._outgoing.get(e).add(a),this._incoming.get(t).add(a),this._edgeProperties.set(a,{...r,weight:n}),a}removeEdge(e){let t=this._edges.get(e);if(t===void 0)throw new Ge(e);this._outgoing.get(t.from).delete(e),this._incoming.get(t.to).delete(e),this._edges.delete(e),this._edgeProperties.delete(e)}hasEdge(e){return this._edges.has(e)}edge(e){let t=this._edges.get(e);if(t===void 0)throw new Ge(e);return t}edges(){return Array.from(this._edges.values())}edgesBetween(e,t){return this.assertNode(e),this.assertNode(t),this.outgoingEdges(e).filter(e=>e.to===t)}outgoingEdges(e){return this.assertNode(e),Array.from(this._outgoing.get(e),e=>this.edge(e))}incomingEdges(e){return this.assertNode(e),Array.from(this._incoming.get(e),e=>this.edge(e))}successors(e){return Array.from(new Set(this.outgoingEdges(e).map(e=>e.to)))}predecessors(e){return Array.from(new Set(this.incomingEdges(e).map(e=>e.from)))}edgeWeight(e){return this.edge(e).weight}graphProperties(){return{...this._graphProperties}}setGraphProperty(e,t){this._graphProperties[e]=t}removeGraphProperty(e){delete this._graphProperties[e]}nodeProperties(e){return this.assertNode(e),{...this._nodeProperties.get(e)??{}}}setNodeProperty(e,t,n){this.assertNode(e),this._nodeProperties.get(e)[t]=n}removeNodeProperty(e,t){this.assertNode(e),delete this._nodeProperties.get(e)[t]}edgeProperties(e){return this.assertEdge(e),{...this._edgeProperties.get(e)??{},weight:this.edgeWeight(e)}}setEdgeProperty(e,t,n){if(this.assertEdge(e),t===`weight`){if(typeof n!=`number`||Number.isNaN(n))throw Error(`Edge property 'weight' must be a number`);this.setEdgeWeight(e,n)}this._edgeProperties.get(e)[t]=n}removeEdgeProperty(e,t){if(this.assertEdge(e),t===`weight`){this.setEdgeWeight(e,1),this._edgeProperties.get(e).weight=1;return}delete this._edgeProperties.get(e)[t]}topologicalSort(){let e=new Map;for(let t of this._nodes)e.set(t,this._incoming.get(t).size);let t=Array.from(this._nodes).filter(t=>e.get(t)===0).sort(Ue),n=[];for(;t.length>0;){let r=t.shift();n.push(r);for(let n of this.outgoingEdges(r)){let r=e.get(n.to)-1;e.set(n.to,r),r===0&&(t.push(n.to),t.sort(Ue))}}if(n.length!==this._nodes.size)throw new qe(`Graph contains a cycle: processed ${n.length}/${this._nodes.size} nodes`);return n}hasCycle(){try{return this.topologicalSort(),!1}catch(e){if(e instanceof qe)return!0;throw e}}independentGroups(){let e=new Map;for(let t of this._nodes)e.set(t,this._incoming.get(t).size);let t=Array.from(this._nodes).filter(t=>e.get(t)===0).sort(Ue),n=[],r=0;for(;t.length>0;){n.push(t),r+=t.length;let i=new Set;for(let n of t)for(let t of this.outgoingEdges(n)){let n=e.get(t.to)-1;e.set(t.to,n),n===0&&i.add(t.to)}t=Array.from(i).sort(Ue)}if(r!==this._nodes.size)throw new qe(`Graph contains a cycle: processed ${r}/${this._nodes.size} nodes`);return n}toString(){return`MultiDirectedGraph(nodes=${this.size}, edges=${this._edges.size})`}allocateEdgeId(){let e=`e${this._nextEdgeId}`;for(;this._edges.has(e);)this._nextEdgeId+=1,e=`e${this._nextEdgeId}`;return this._nextEdgeId+=1,e}assertNode(e){if(!this._nodes.has(e))throw new We(e)}assertEdge(e){if(!this._edges.has(e))throw new Ge(e)}validateWeight(e){if(typeof e!=`number`||Number.isNaN(e))throw Error(`Edge weight must be a number`)}setEdgeWeight(e,t){this.validateWeight(t);let n=this.edge(e);this._edges.set(e,{...n,weight:t})}},Ye=class{graph;constructor(e,t=Qe(e)){this.graph=t}input(e,t=e,n={}){return $e(this.graph,e,t,n),this}constant(e,t,n={}){return et(this.graph,e,t,n),this}weightedSum(e,t,n={}){return tt(this.graph,e,t,n),this}activation(e,t,n,r={},i){return nt(this.graph,e,t,n,r,i),this}output(e,t,n=e,r={},i){return rt(this.graph,e,t,n,r,i),this}};function Xe(e){return new Ye(e)}function Ze(e){if(e.inputNames.length===0)throw Error(`feed-forward network must have at least one input`);if(e.layers.length===0)throw Error(`feed-forward network must have at least one layer`);let t=Xe(e.name),n=`bias`;t.constant(n,1,{"nn.role":`bias`});let r=e.inputNames.map((e,n)=>{let r=`input_${n}`;return t.input(r,e,{"nn.layer":`input`,"nn.index":n}),r});for(let[i,a]of e.layers.entries()){let o=a.name??`layer_${i}`;it(a,r.length,o);let s=[];for(let e=0;e<a.biases.length;e+=1){let i=`${o}_${e}_sum`,c=`${o}_${e}`;t.weightedSum(i,[...r.map((t,n)=>({from:t,weight:a.weights[n][e],edgeId:`${t}_to_${i}`,properties:{"nn.trainable":!0,"nn.layer":o}})),{from:n,weight:a.biases[e],edgeId:`${n}_to_${i}`,properties:{"nn.trainable":!0,"nn.role":`bias_weight`,"nn.layer":o}}],{"nn.layer":o,"nn.index":e,"nn.role":`weighted_sum`}).activation(c,i,a.activation??`none`,{"nn.layer":o,"nn.index":e,"nn.role":`activation`},`${i}_to_${c}`),s.push(c)}if(i===e.layers.length-1)for(let[e,n]of s.entries()){let r=a.outputNames?.[e]??(s.length===1?`prediction`:`output${e}`);t.output(`${o}_${e}_out`,n,r,{"nn.layer":o,"nn.index":e,"nn.role":`output`},`${n}_to_${o}_${e}_out`)}r=s}return t}function Qe(e){let t=new Je;return t.setGraphProperty(`nn.version`,`0`),e!==void 0&&t.setGraphProperty(`nn.name`,e),t}function $e(e,t,n=t,r={}){e.addNode(t,{...r,"nn.op":`input`,"nn.input":n})}function et(e,t,n,r={}){if(!Number.isFinite(n))throw Error(`constant value must be finite`);e.addNode(t,{...r,"nn.op":`constant`,"nn.value":n})}function tt(e,t,n,r={}){e.addNode(t,{...r,"nn.op":`weighted_sum`});for(let r of n)e.addEdge(r.from,t,r.weight??1,r.properties??{},r.edgeId)}function nt(e,t,n,r,i={},a){return e.addNode(t,{...i,"nn.op":`activation`,"nn.activation":r}),e.addEdge(n,t,1,{},a)}function rt(e,t,n,r=t,i={},a){return e.addNode(t,{...i,"nn.op":`output`,"nn.output":r}),e.addEdge(n,t,1,{},a)}function it(e,t,n){if(e.biases.length===0)throw Error(`${n} must have at least one unit`);if(e.weights.length!==t)throw Error(`${n} weight row count must match previous layer width`);if(e.outputNames!==void 0&&e.outputNames.length!==e.biases.length)throw Error(`${n} output name count must match unit count`);for(let[t,r]of e.weights.entries()){if(r.length!==e.biases.length)throw Error(`${n} weight row ${t} width must match bias count`);for(let e of r)if(!Number.isFinite(e))throw Error(`${n} weights must be finite`)}for(let t of e.biases)if(!Number.isFinite(t))throw Error(`${n} biases must be finite`)}var at=class extends Error{nodeId;edgeId;constructor(e,t,n){super(e),this.nodeId=t,this.edgeId=n,this.name=`NeuralGraphCompileError`}};function ot(e){let t=e.topologicalSort(),n=[],r=new Map,i=0,a=()=>`v${i++}`;for(let i of t){let t=e.nodeProperties(i),o=ft(t[`nn.op`],`weighted_sum`);if(o===`input`){let e=a();r.set(i,e),n.push({op:`LOAD_INPUT`,dst:e,inputName:ft(t[`nn.input`],i),sourceNode:i});continue}if(o===`constant`){let e=a();r.set(i,e),n.push({op:`LOAD_CONST`,dst:e,value:pt(t[`nn.value`],i,`nn.value`),sourceNode:i});continue}if(o===`weighted_sum`){let t=[];for(let o of e.incomingEdges(i).sort(dt)){let e=r.get(o.from);if(e===void 0)throw new at(`Source node has no value: ${o.from}`,o.from,o.id);let i=a(),s=a();n.push({op:`LOAD_EDGE_WEIGHT`,dst:i,edgeId:o.id,sourceEdge:o.id}),n.push({op:`MUL`,dst:s,left:e,right:i,sourceEdge:o.id}),t.push(s)}let o=a();r.set(i,o),n.push({op:t.length===0?`LOAD_CONST`:`ADD`,dst:o,value:t.length===0?0:void 0,inputs:t.length===0?void 0:t,sourceNode:i});continue}if(o===`activation`){let o=ut(e,r,i),s=a();r.set(i,s),n.push({op:`ACTIVATE`,dst:s,input:o,activation:ft(t[`nn.activation`],`relu`),sourceNode:i});continue}if(o===`output`){let a=ut(e,r,i);r.set(i,a),n.push({op:`STORE_OUTPUT`,outputName:ft(t[`nn.output`],i),input:a,sourceNode:i});continue}throw new at(`Unsupported neural graph op: ${o}`,i)}return{magic:`CANN`,version:0,graph:{nodes:e.nodes(),edges:e.edges().map(e=>({id:e.id,from:e.from,to:e.to,weight:e.weight}))},functions:[{id:`forward`,kind:`forward`,instructions:n}]}}function st(e){return ot(e.graph)}function ct(e,t){return lt(e,t,!0)}function lt(e,t,n){let r=new Map,i=new Map(e.graph.edges.map(e=>[e.id,e.weight])),a={},o=[],s=e.functions.find(e=>e.kind===`forward`);if(s===void 0)throw Error(`Neural bytecode module has no forward function`);for(let[e,c]of s.instructions.entries()){let s=[],l,u,d=e=>{let t=gt(r,e);return s.push({valueId:e,value:t}),t},f=(e,t)=>{r.set(e,t),l={valueId:e,value:t}};switch(c.op){case`LOAD_INPUT`:mt(c),f(c.dst,ht(t,c.inputName));break;case`LOAD_CONST`:mt(c),f(c.dst,c.value??0);break;case`LOAD_EDGE_WEIGHT`:mt(c),f(c.dst,i.get(c.edgeId??``)??1);break;case`MUL`:mt(c),f(c.dst,d(c.left)*d(c.right));break;case`ADD`:mt(c),f(c.dst,(c.inputs??[]).reduce((e,t)=>e+d(t),0));break;case`ACTIVATE`:mt(c),f(c.dst,_t(d(c.input),c.activation??`relu`));break;case`STORE_OUTPUT`:u={outputName:c.outputName??`output`,value:d(c.input)},a[u.outputName]=u.value;break}n&&o.push({index:e,instruction:c,reads:s,write:l,output:u,sourceNode:c.sourceNode,sourceEdge:c.sourceEdge})}return{outputs:a,values:Object.fromEntries(r),instructions:o}}function ut(e,t,n){let r=e.incomingEdges(n).sort(dt);if(r.length!==1)throw new at(`Expected exactly one input edge for ${n}, got ${r.length}`,n);let i=t.get(r[0].from);if(i===void 0)throw new at(`Source node has no value: ${r[0].from}`,r[0].from,r[0].id);return i}function dt(e,t){return e.id.localeCompare(t.id)}function ft(e,t){return typeof e==`string`?e:t}function pt(e,t,n){if(typeof e!=`number`||!Number.isFinite(e))throw new at(`Expected numeric property ${n} on ${t}`,t);return e}function mt(e){if(e.dst===void 0)throw Error(`Instruction ${e.op} is missing dst`)}function ht(e,t){if(t===void 0||!(t in e))throw Error(`Missing input: ${t??`<undefined>`}`);return e[t]}function gt(e,t){if(t===void 0||!e.has(t))throw Error(`Missing value: ${t??`<undefined>`}`);return e.get(t)}function _t(e,t){switch(t){case`relu`:return Math.max(0,e);case`sigmoid`:return 1/(1+Math.exp(-Math.max(-500,Math.min(500,e))));case`tanh`:return Math.tanh(e);case`none`:return e;default:return e}}var vt=class{name=`cpu`;add(e,t){return e.add(t)}subtract(e,t){return e.subtract(t)}scale(e,t){return e.scale(t)}transpose(e){return e.transpose()}dot(e,t){return e.dot(t)}};function yt(e,t,n){if(!Number.isSafeInteger(e)||e<0||e>=t)throw Error(`${n} index ${String(e)} out of bounds for size ${t}.`)}new vt;var bt=class e{data;rows;cols;constructor(e){typeof e==`number`?this.data=[[e]]:Array.isArray(e)&&e.length>0&&typeof e[0]==`number`?this.data=[e]:Array.isArray(e)?this.data=e:this.data=[],this.rows=this.data.length,this.cols=this.rows>0?this.data[0].length:0}static zeros(t,n){return new e(Array.from({length:t},()=>Array(n).fill(0)))}static identity(t){return new e(Array.from({length:t},(e,n)=>Array.from({length:t},(e,t)=>+(n===t))))}static fromDiagonal(t){let n=t.length;return new e(Array.from({length:n},(e,r)=>Array.from({length:n},(e,n)=>r===n?t[r]:0)))}add(t){if(typeof t==`number`)return new e(this.data.map(e=>e.map(e=>e+t)));if(this.rows!==t.rows||this.cols!==t.cols)throw Error(`Add dimension mismatch.`);return new e(this.data.map((e,n)=>e.map((e,r)=>e+t.data[n][r])))}subtract(t){if(typeof t==`number`)return new e(this.data.map(e=>e.map(e=>e-t)));if(this.rows!==t.rows||this.cols!==t.cols)throw Error(`Subtract dimension mismatch.`);return new e(this.data.map((e,n)=>e.map((e,r)=>e-t.data[n][r])))}scale(t){return new e(this.data.map(e=>e.map(e=>e*t)))}transpose(){return this.rows===0?new e([]):new e(this.data[0].map((e,t)=>this.data.map(e=>e[t])))}dot(t){if(this.cols!==t.rows)throw Error(`Dot product inner dimensions strictly mismatch.`);let n=e.zeros(this.rows,t.cols);for(let e=0;e<this.rows;e++)for(let r=0;r<t.cols;r++)for(let i=0;i<this.cols;i++)n.data[e][r]+=this.data[e][i]*t.data[i][r];return n}get(e,t){return yt(e,this.rows,`row`),yt(t,this.cols,`col`),this.data[e][t]}set(t,n,r){yt(t,this.rows,`row`),yt(n,this.cols,`col`);let i=[...this.data[t]];return i.splice(n,1,r),new e(this.data.map((e,n)=>n===t?i:[...e]))}sum(){let e=0;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)e+=this.data[t][n];return e}sumRows(){return new e(this.data.map(e=>[e.reduce((e,t)=>e+t,0)]))}sumCols(){let t=Array(this.cols).fill(0);for(let e=0;e<this.rows;e++)for(let n=0;n<this.cols;n++)t[n]+=this.data[e][n];return new e([t])}mean(){return this.sum()/(this.rows*this.cols)}min(){let e=1/0;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)this.data[t][n]<e&&(e=this.data[t][n]);return e}max(){let e=-1/0;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)this.data[t][n]>e&&(e=this.data[t][n]);return e}argmin(){let e=1/0,t=0,n=0;for(let r=0;r<this.rows;r++)for(let i=0;i<this.cols;i++)this.data[r][i]<e&&(e=this.data[r][i],t=r,n=i);return[t,n]}argmax(){let e=-1/0,t=0,n=0;for(let r=0;r<this.rows;r++)for(let i=0;i<this.cols;i++)this.data[r][i]>e&&(e=this.data[r][i],t=r,n=i);return[t,n]}map(t){return new e(this.data.map(e=>e.map(t)))}sqrt(){return this.map(Math.sqrt)}abs(){return this.map(Math.abs)}pow(e){return this.map(t=>t**+e)}flatten(){let t=[];for(let e=0;e<this.rows;e++)for(let n=0;n<this.cols;n++)t.push(this.data[e][n]);return new e([t])}reshape(t,n){if(t*n!==this.rows*this.cols)throw Error(`Cannot reshape ${this.rows}x${this.cols} to ${t}x${n}.`);let r=this.flatten().data[0],i=[];for(let e=0;e<t;e++)i.push(r.slice(e*n,(e+1)*n));return new e(i)}row(t){if(t<0||t>=this.rows)throw Error(`Row index ${t} out of bounds for ${this.rows} rows.`);return new e([[...this.data[t]]])}col(t){if(t<0||t>=this.cols)throw Error(`Column index ${t} out of bounds for ${this.cols} cols.`);return new e(this.data.map(e=>[e[t]]))}slice(t,n,r,i){if(t<0||n>this.rows||r<0||i>this.cols||t>=n||r>=i)throw Error(`Invalid slice [${t}:${n}, ${r}:${i}] for ${this.rows}x${this.cols} matrix.`);let a=[];for(let e=t;e<n;e++)a.push(this.data[e].slice(r,i));return new e(a)}equals(e){if(this.rows!==e.rows||this.cols!==e.cols)return!1;for(let t=0;t<this.rows;t++)for(let n=0;n<this.cols;n++)if(this.data[t][n]!==e.data[t][n])return!1;return!0}close(e,t=1e-9){if(this.rows!==e.rows||this.cols!==e.cols)return!1;for(let n=0;n<this.rows;n++)for(let r=0;r<this.cols;r++)if(Math.abs(this.data[n][r]-e.data[n][r])>t)return!1;return!0}},xt=class{fromRows(e){return new bt(Pt(e))}toRows(e){return Pt(e.data)}column(e){return new bt(e.map(e=>[e]))}constant(e,t,n=1){return new bt(Array.from({length:t},()=>Array(n).fill(e)))}add(e,t){return e.add(t)}scale(e,t){return e.scale(t)}dot(e,t){return e.dot(t)}map(e,t){return e.map(t)}toColumn(e){if(e.cols!==1)throw Error(`Expected a single-column matrix, got ${e.cols} columns`);return e.data.map(e=>e[0])}},St=class{backend=new xt;column(e){return this.backend.column(e)}constant(e,t,n=1){return this.backend.constant(e,t,n)}add(e,t){return this.backend.add(e,t)}scale(e,t){return this.backend.scale(e,t)}activate(e,t){return this.backend.map(e,e=>_t(e,t))}toColumn(e){return this.backend.toColumn(e)}};function Ct(e){let t=e.functions.find(e=>e.kind===`forward`);if(t===void 0)throw Error(`Neural bytecode module has no forward function`);let n=new Map(e.graph.edges.map(e=>[e.id,e.weight])),r=new Map,i=new Map,a=new Map,o=[];for(let[e,s]of t.instructions.entries())switch(s.op){case`LOAD_INPUT`:{let t=kt(s);r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`LOAD_INPUT_MATRIX`,dst:t,inputName:s.inputName,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`LOAD_CONST`:{let t=kt(s);r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`LOAD_CONST_MATRIX`,dst:t,value:s.value??0,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`LOAD_EDGE_WEIGHT`:{let e=kt(s),t=At(s);i.set(e,{valueId:e,edgeId:t,weight:n.get(t)??1});break}case`MUL`:{let e=kt(s),t=Et(s,r,i);a.set(e,t);break}case`ADD`:{let t=kt(s),n=(s.inputs??[]).map(e=>{let t=a.get(e);if(t===void 0)throw Error(`Cannot lower ADD input ${e} to a matrix term`);return t});r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`WEIGHTED_SUM_MATRIX`,dst:t,terms:n,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`ACTIVATE`:{let t=kt(s);jt(s.input,r),r.set(t,{valueId:t,sourceNode:s.sourceNode}),o.push({op:`ACTIVATE_MATRIX`,dst:t,input:s.input,activation:s.activation??`relu`,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}case`STORE_OUTPUT`:jt(s.input,r),o.push({op:`STORE_OUTPUT_MATRIX`,outputName:s.outputName??`output`,input:s.input,sourceNode:s.sourceNode,sourceInstructionIndexes:[e]});break}return{magic:`CANM`,version:0,sourceBytecodeVersion:e.version,instructions:o}}async function wt(e,t,n){let r=n??new St,i=Dt(t),a=new Map,o=new Set,s={},c=e=>(o.add(e),e);try{for(let n of e.instructions)switch(n.op){case`LOAD_INPUT_MATRIX`:{let e=Mt(n),o=n.inputName??n.sourceNode??e;a.set(e,c(await r.column(Ot(t,o,i))));break}case`LOAD_CONST_MATRIX`:{let e=Mt(n);a.set(e,c(await r.constant(n.value??0,i)));break}case`WEIGHTED_SUM_MATRIX`:{let e=Mt(n),t=n.terms??[],o;for(let e of t){let t=c(await r.scale(Nt(a,e.sourceValue),e.weight));o=o===void 0?t:c(await r.add(o,t))}a.set(e,o??c(await r.constant(0,i)));break}case`ACTIVATE_MATRIX`:{let e=Mt(n);a.set(e,c(await r.activate(Nt(a,n.input),n.activation??`relu`)));break}case`STORE_OUTPUT_MATRIX`:{let e=n.outputName??`output`;s[e]=await r.toColumn(Nt(a,n.input));break}}return{outputs:s,values:Object.fromEntries(await Promise.all([...a.entries()].map(async([e,t])=>[e,await r.toColumn(t)])))}}finally{r.dispose!==void 0&&await Promise.all([...o].map(e=>r.dispose(e)))}}function Tt(e,t,n=new xt){let r=Dt(t),i=new Map,a={};for(let o of e.instructions)switch(o.op){case`LOAD_INPUT_MATRIX`:{let e=Mt(o),a=o.inputName??o.sourceNode??e;i.set(e,n.column(Ot(t,a,r)));break}case`LOAD_CONST_MATRIX`:{let e=Mt(o);i.set(e,n.constant(o.value??0,r));break}case`WEIGHTED_SUM_MATRIX`:{let e=Mt(o),t=o.terms??[],a;for(let e of t){let t=n.scale(Nt(i,e.sourceValue),e.weight);a=a===void 0?t:n.add(a,t)}i.set(e,a??n.constant(0,r));break}case`ACTIVATE_MATRIX`:{let e=Mt(o);i.set(e,n.map(Nt(i,o.input),e=>_t(e,o.activation??`relu`)));break}case`STORE_OUTPUT_MATRIX`:{let e=o.outputName??`output`;a[e]=n.toColumn(Nt(i,o.input));break}}return{outputs:a,values:Object.fromEntries([...i.entries()].map(([e,t])=>[e,n.toColumn(t)]))}}function Et(e,t,n){let r=t.get(e.left??``),i=t.get(e.right??``),a=n.get(e.left??``),o=n.get(e.right??``);if(r!==void 0&&o!==void 0)return{sourceValue:r.valueId,sourceNode:r.sourceNode,edgeId:o.edgeId,weight:o.weight};if(i!==void 0&&a!==void 0)return{sourceValue:i.valueId,sourceNode:i.sourceNode,edgeId:a.edgeId,weight:a.weight};throw Error(`Cannot lower MUL ${e.dst??`<unknown>`} to a weighted matrix term`)}function Dt(e){let t=1;for(let n of Object.values(e))if(Array.isArray(n)){if(n.length===0)throw Error(`Batched inputs must contain at least one value`);if(t!==1&&n.length!==t)throw Error(`All batched inputs must have the same length`);t=n.length}return t}function Ot(e,t,n){if(!(t in e))throw Error(`Missing input: ${t}`);let r=e[t];if(Array.isArray(r)){if(r.length!==n)throw Error(`All batched inputs must have the same length`);return[...r]}return Array(n).fill(r)}function kt(e){if(e.dst===void 0)throw Error(`Instruction ${e.op} is missing dst`);return e.dst}function At(e){if(e.edgeId===void 0)throw Error(`LOAD_EDGE_WEIGHT is missing edgeId`);return e.edgeId}function jt(e,t){if(e===void 0||!t.has(e))throw Error(`Cannot lower missing value: ${e??`<undefined>`}`)}function Mt(e){if(e.dst===void 0)throw Error(`Matrix plan instruction ${e.op} is missing dst`);return e.dst}function Nt(e,t){if(t===void 0||!e.has(t))throw Error(`Missing matrix value: ${t??`<undefined>`}`);return e.get(t)}function Pt(e){return e.map(e=>[...e])}var Ft={MAP_READ:1,COPY_SRC:4,COPY_DST:8,STORAGE:128},It={READ:1},Lt={COMPUTE:4},Rt=64,zt=class e{device;unaryLayout;binaryLayout;scalePipeline;addPipeline;activationPipeline;constructor(e){this.device=e,this.unaryLayout=this.device.createBindGroupLayout({label:`neural-matrix-unary-layout`,entries:[Vt(0,`read-only-storage`),Vt(1,`read-only-storage`),Vt(2,`storage`)]}),this.binaryLayout=this.device.createBindGroupLayout({label:`neural-matrix-binary-layout`,entries:[Vt(0,`read-only-storage`),Vt(1,`read-only-storage`),Vt(2,`storage`)]}),this.scalePipeline=this.createPipeline(`neural-matrix-scale`,qt,this.unaryLayout),this.addPipeline=this.createPipeline(`neural-matrix-add`,Kt,this.binaryLayout),this.activationPipeline=this.createPipeline(`neural-matrix-activation`,Jt,this.unaryLayout)}static async create(t,n={}){let r=await t.requestAdapter(n);if(r===null)throw Error(`WebGPU is available, but no adapter was returned`);return e.createFromAdapter(r)}static async createFromNavigator(t={}){let n=Bt();if(n===void 0)return null;let r=await n.requestAdapter(t);return r===null?null:e.createFromAdapter(r)}static isNavigatorAvailable(){return Bt()!==void 0}static async createFromAdapter(t){let n=await t.requestDevice();try{return new e(n)}catch(e){throw n.destroy?.(),e}}async fromRows(e){let t=e.length,n=e[0]?.length??0,r=new Float32Array(t*n);return e.forEach((e,t)=>{if(e.length!==n)throw Error(`All WebGPU matrix rows must have the same column count`);e.forEach((e,i)=>{r[t*n+i]=e})}),this.upload(r,t,n,`neural-matrix-rows`)}async toRows(e){let t=await this.download(e);return Array.from({length:e.rows},(n,r)=>Array.from(t.slice(r*e.cols,r*e.cols+e.cols)))}column(e){return this.upload(new Float32Array(e),e.length,1,`neural-matrix-column`)}constant(e,t,n=1){return this.upload(new Float32Array(t*n).fill(e),t,n,`neural-matrix-constant`)}add(e,t){Wt(e,t);let n=this.createOutput(e.rows,e.cols,`neural-matrix-add-output`);return this.runBinary(this.addPipeline,e,t,n,`neural-matrix-add-pass`),n}scale(e,t){let n=this.uploadParameter(new Float32Array([t]),`neural-matrix-scale-value`),r=this.createOutput(e.rows,e.cols,`neural-matrix-scale-output`,[n]);return this.runUnary(this.scalePipeline,e,n,r,`neural-matrix-scale-pass`),r}activate(e,t){let n=this.uploadParameter(new Uint32Array([Gt(t)]),`neural-matrix-activation-code`),r=this.createOutput(e.rows,e.cols,`neural-matrix-activation-output`,[n]);return this.runUnary(this.activationPipeline,e,n,r,`neural-matrix-activation-pass`),r}async toColumn(e){if(e.cols!==1)throw Error(`Expected a single-column WebGPU matrix, got ${e.cols} columns`);return Array.from(await this.download(e))}dispose(e){e.buffer.destroy?.(),e.scratch?.forEach(e=>e.destroy?.())}destroy(){this.device.destroy?.()}createPipeline(e,t,n){let r=this.device.createShaderModule({label:`${e}-shader`,code:t}),i=this.device.createPipelineLayout({label:`${e}-pipeline-layout`,bindGroupLayouts:[n]});return this.device.createComputePipeline({label:e,layout:i,compute:{module:r,entryPoint:`main`}})}upload(e,t,n,r){let i=Ut(e.length),a=this.device.createBuffer({label:r,size:i,usage:Ft.STORAGE|Ft.COPY_SRC|Ft.COPY_DST});return e.length>0&&this.device.queue.writeBuffer(a,0,e),{rows:t,cols:n,length:e.length,byteLength:i,buffer:a}}uploadParameter(e,t){let n=this.device.createBuffer({label:t,size:Ut(e.length),usage:Ft.STORAGE|Ft.COPY_DST});return this.device.queue.writeBuffer(n,0,e),n}createOutput(e,t,n,r=[]){let i=e*t,a=Ut(i);return{rows:e,cols:t,length:i,byteLength:a,buffer:this.device.createBuffer({label:n,size:a,usage:Ft.STORAGE|Ft.COPY_SRC|Ft.COPY_DST}),scratch:r}}runBinary(e,t,n,r,i){let a=this.device.createBindGroup({label:`${i}-bind-group`,layout:this.binaryLayout,entries:[Ht(0,t.buffer),Ht(1,n.buffer),Ht(2,r.buffer)]});this.dispatch(e,a,r.length,i)}runUnary(e,t,n,r,i){let a=this.device.createBindGroup({label:`${i}-bind-group`,layout:this.unaryLayout,entries:[Ht(0,t.buffer),Ht(1,n),Ht(2,r.buffer)]});this.dispatch(e,a,r.length,i)}dispatch(e,t,n,r){let i=this.device.createCommandEncoder({label:`${r}-encoder`}),a=i.beginComputePass({label:r});a.setPipeline(e),a.setBindGroup(0,t),a.dispatchWorkgroups(Math.max(1,Math.ceil(n/Rt))),a.end(),this.device.queue.submit([i.finish()])}async download(e){let t=this.device.createBuffer({label:`neural-matrix-readback`,size:e.byteLength,usage:Ft.MAP_READ|Ft.COPY_DST}),n=this.device.createCommandEncoder({label:`neural-matrix-readback-encoder`});n.copyBufferToBuffer(e.buffer,0,t,0,e.byteLength),this.device.queue.submit([n.finish()]),await this.device.queue.onSubmittedWorkDone?.(),await t.mapAsync(It.READ,0,e.byteLength);let r=t.getMappedRange(0,e.byteLength),i=(ArrayBuffer.isView(r)?new Uint8Array(r.buffer,r.byteOffset,r.byteLength):new Uint8Array(r)).slice(0,e.length*Float32Array.BYTES_PER_ELEMENT),a=new Float32Array(i.buffer,i.byteOffset,e.length),o=new Float32Array(a);return t.unmap(),t.destroy?.(),o}};function Bt(){return globalThis.navigator?.gpu}function Vt(e,t){return{binding:e,visibility:Lt.COMPUTE,buffer:{type:t}}}function Ht(e,t){return{binding:e,resource:{buffer:t}}}function Ut(e){return Math.max(Float32Array.BYTES_PER_ELEMENT,e*Float32Array.BYTES_PER_ELEMENT)}function Wt(e,t){if(e.rows!==t.rows||e.cols!==t.cols)throw Error(`WebGPU matrix shape mismatch: ${e.rows}x${e.cols} vs ${t.rows}x${t.cols}`)}function Gt(e){switch(e){case`none`:case`linear`:return 0;case`relu`:return 1;case`sigmoid`:return 2;case`tanh`:return 3;default:throw Error(`Unsupported WebGPU activation: ${e}`)}}var Kt=`
@group(0) @binding(0) var<storage, read> left_values: array<f32>;
@group(0) @binding(1) var<storage, read> right_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(${Rt})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = left_values[index] + right_values[index];
}
`,qt=`
@group(0) @binding(0) var<storage, read> input_values: array<f32>;
@group(0) @binding(1) var<storage, read> scalar_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(${Rt})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = input_values[index] * scalar_values[0];
}
`,Jt=`
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

@compute @workgroup_size(${Rt})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = apply_activation(input_values[index], activation_values[0]);
}
`,Yt=8,Xt=512,Zt=1e3,Qt=0xe8d4a51000,$t=1e-5,en=[{id:`one_row_by_hand`,title:`One row by hand`,summary:`Save one forward row, reverse it, then apply one scalar SGD update.`,initialParameter:.5,learningRate:.1,inputs:[2],targets:[0],gradientBufferBefore:0,divisor:1},{id:`two_row_mean`,title:`The same plan, two-row mean`,summary:`Keep every instruction ID fixed while two row gradients reduce and average.`,initialParameter:1,learningRate:.1,inputs:[2,-1],targets:[1,1],gradientBufferBefore:0,divisor:2},{id:`persistent_buffer`,title:`Continue a persistent buffer`,summary:`Enter with grad_w = 3, add one new row gradient of 2, and keep 5 after SGD.`,initialParameter:.5,learningRate:.1,inputs:[2],targets:[0],gradientBufferBefore:3,divisor:1}];function k(e,t,n=!0){let r=n?Qt:Zt;if(!Number.isFinite(e)||Math.abs(e)>r)throw Error(`${t} must be finite and bounded by ${r}`);return e}function tn(e,t){if(typeof e!=`string`||e.length<1||e.length>Xt)throw Error(`${t} must be a bounded string`);return e}function nn(e,t,n){let r=Object.keys(e).sort(),i=[...t].sort();if(r.length!==i.length||r.some((e,t)=>e!==i[t]))throw Error(`${n} must contain exactly ${i.join(`, `)}`)}function rn(e,t){if(!Array.isArray(e)||e.length<1||e.length>Yt)throw Error(`${t} must contain 1 to ${Yt} values`);return e.map((e,n)=>{if(typeof e!=`number`)throw Error(`${t}[${n}] must be numeric`);return k(e,`${t}[${n}]`,!1)})}function an(e){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`scenario must be an object`);nn(e,[`id`,`title`,`summary`,`initialParameter`,`learningRate`,`inputs`,`targets`,`gradientBufferBefore`,`divisor`],`scenario`);let t=rn(e.inputs,`scenario.inputs`),n=rn(e.targets,`scenario.targets`);if(t.length!==n.length)throw Error(`inputs and targets must have the same bounded length`);let r=e.divisor;if(!Number.isInteger(r)||r<1||r>t.length)throw Error(`divisor must be an integer within the batch length`);let i=k(e.initialParameter,`initial parameter`,!1),a=k(e.learningRate,`learning rate`,!1),o=k(e.gradientBufferBefore,`gradient buffer before`,!1);if(a<=0)throw Error(`learning rate must be positive`);return{id:tn(e.id,`scenario.id`),title:tn(e.title,`scenario.title`),summary:tn(e.summary,`scenario.summary`),initialParameter:i,learningRate:a,inputs:t,targets:n,gradientBufferBefore:o,divisor:r}}function on(e,t,n,r,i={},a=[],o=[],s=[]){return{id:e,op:t,output:n,inputs:r,attributes:i,sourceNodes:a,sourceEdges:o,sourceInstructions:s}}function sn(){return{magic:`CANB`,version:0,instructions:[on(`b0`,`SEED_LOSS_GRAD`,`d_loss`,[],{value:1},[`loss`]),on(`b1`,`HALF_SQUARED_ERROR_GRAD`,`d_residual`,[`residual`,`d_loss`],{},[`loss`,`residual`]),on(`b2`,`PROPAGATE_GRAD`,`d_prediction`,[`d_residual`],{through:`subtract_prediction`},[`residual`,`prediction`]),on(`b3`,`PARAMETER_LOCAL_GRAD`,`local_d_w`,[`x`,`d_prediction`],{parameter_id:`w`},[`prediction`],[`w`]),on(`b4`,`ACCUMULATE_GRAD`,`grad_w`,[`grad_w`,`local_d_w`],{parameter_id:`w`,order:`row_ascending`},[],[`w`]),on(`b5`,`INPUT_GRAD`,`d_x`,[`w`,`d_prediction`],{input_id:`x`},[`x`,`prediction`],[`w`])]}}function cn(){return{magic:`CANO`,version:0,instructions:[on(`o0`,`READ_GRAD_BUFFER`,`total_d_w`,[`grad_w`],{parameter_id:`w`},[],[`w`]),on(`o1`,`DIVIDE_GRAD`,`applied_d_w`,[`total_d_w`],{divisor_source:`scenario.divisor`},[],[`w`],[`o0`]),on(`o2`,`SGD_UPDATE`,`w_next`,[`w`,`applied_d_w`],{learning_rate_source:`scenario.learning_rate`},[],[`w`],[`o1`]),on(`o3`,`KEEP_GRAD_BUFFER`,`grad_w_after_step`,[`grad_w`],{optimizer_step_zeroes_gradient:!1},[],[`w`],[`o2`])]}}function ln(){return{magic:`CANM-TRAIN`,version:0,instructions:[on(`t0`,`LOAD_SAVED_COLUMN`,`x_col`,[`x`],{saved_value:`x`},[`x`]),on(`t1`,`LOAD_SAVED_COLUMN`,`residual_col`,[`residual`],{saved_value:`residual`},[`residual`]),on(`t2`,`LOSS_GRAD_COLUMN`,`d_prediction_col`,[`residual_col`],{loss:`half_squared_error`},[`loss`,`prediction`],[],[`b0`,`b1`,`b2`]),on(`t3`,`PARAMETER_LOCAL_GRAD_COLUMN`,`local_d_w_col`,[`x_col`,`d_prediction_col`],{parameter_id:`w`},[`prediction`],[`w`],[`b3`]),on(`t4`,`INPUT_GRAD_COLUMN`,`d_x_col`,[`d_prediction_col`],{input_id:`x`,parameter_id:`w`},[`x`,`prediction`],[`w`],[`b5`]),on(`t5`,`REDUCE_SUM_GRAD`,`batch_d_w`,[`local_d_w_col`],{order:`row_ascending`,parameter_id:`w`},[],[`w`],[`b4`]),on(`t6`,`ACCUMULATE_GRAD_BUFFER`,`grad_w`,[`grad_w`,`batch_d_w`],{parameter_id:`w`},[],[`w`],[`b4`]),on(`t7`,`DIVIDE_GRAD`,`applied_d_w`,[`grad_w`],{divisor_source:`scenario.divisor`},[],[`w`],[`o0`,`o1`]),on(`t8`,`SGD_UPDATE_SCALAR`,`w_next`,[`w`,`applied_d_w`],{learning_rate_source:`scenario.learning_rate`},[],[`w`],[`o2`]),on(`t9`,`KEEP_GRAD_BUFFER`,`grad_w_after_step`,[`grad_w`],{optimizer_step_zeroes_gradient:!1},[],[`w`],[`o3`])]}}function un(e){let t=Qe(`nn30_scalar_training`);return $e(t,`x`,`x`),tt(t,`prediction_sum`,[{from:`x`,weight:e,edgeId:`w`,properties:{"nn.trainable":!0}}]),nt(t,`prediction`,`prediction_sum`,`none`,{},`sum_to_prediction`),rt(t,`out`,`prediction`,`prediction`,{},`prediction_to_out`),t}function dn(e){let t=ot(un(e.initialParameter)),n=Ct(t),r=e.inputs.map(e=>k(ct(t,{x:e}).outputs.prediction,`NeuralIR prediction`)),i=Tt(n,{x:e.inputs}).outputs.prediction.map(e=>k(e,`MatrixIR prediction`)),a=e.inputs.map(t=>k(e.initialParameter*t,`direct prediction`));return{directOutputs:a,neuralIrOutputs:r,matrixIrOutputs:i,neuralOps:t.functions[0].instructions.map(e=>e.op),matrixOps:n.instructions.map(e=>e.op),maxError:xn([a,r,i])}}function fn(e,t){let n=t.map((t,n)=>k(t-e.targets[n],`residual ${n}`)),r=n.map((e,t)=>k(.5*e*e,`loss ${t}`));return{x:[...e.inputs],target:[...e.targets],prediction:[...t],residual:n,loss:r}}function pn(e,t,n,r){let i=new Map;i.set(`x`,[...t.x]),i.set(`residual`,[...t.residual]),i.set(`w`,n),i.set(`grad_w`,r);for(let r of e.instructions)switch(r.op){case`SEED_LOSS_GRAD`:i.set(r.output,Array(t.x.length).fill(1));break;case`HALF_SQUARED_ERROR_GRAD`:{let e=gn(i,`residual`),t=gn(i,`d_loss`);i.set(r.output,e.map((e,n)=>k(e*t[n],`d_residual ${n}`)));break}case`PROPAGATE_GRAD`:i.set(r.output,[...gn(i,`d_residual`)]);break;case`PARAMETER_LOCAL_GRAD`:{let e=gn(i,`x`),t=gn(i,`d_prediction`);i.set(r.output,e.map((e,n)=>k(e*t[n],`local_d_w ${n}`)));break}case`ACCUMULATE_GRAD`:{let e=_n(i,`grad_w`);for(let t of gn(i,`local_d_w`))e=k(e+t,`grad_w reduction`);i.set(r.output,e);break}case`INPUT_GRAD`:i.set(r.output,gn(i,`d_prediction`).map((e,t)=>k(n*e,`d_x ${t}`)));break;default:throw Error(`unsupported backward op: ${r.op}`)}let a=gn(i,`local_d_w`),o=yn(a,`backward batch gradient`);return{dLoss:gn(i,`d_loss`),dResidual:gn(i,`d_residual`),dPrediction:gn(i,`d_prediction`),localDW:a,dX:gn(i,`d_x`),gradientBufferBefore:r,batchGradient:o,gradW:_n(i,`grad_w`)}}function mn(e,t,n){let r=new Map([[`w`,t.initialParameter],[`grad_w`,n]]);for(let n of e.instructions)switch(n.op){case`READ_GRAD_BUFFER`:r.set(n.output,vn(r,`grad_w`));break;case`DIVIDE_GRAD`:r.set(n.output,k(vn(r,`total_d_w`)/t.divisor,`applied gradient`));break;case`SGD_UPDATE`:r.set(n.output,k(vn(r,`w`)-t.learningRate*vn(r,`applied_d_w`),`w_next`));break;case`KEEP_GRAD_BUFFER`:r.set(n.output,vn(r,`grad_w`));break;default:throw Error(`unsupported optimizer op: ${n.op}`)}let i=vn(r,`applied_d_w`),a=vn(r,`w_next`);return{parameterBefore:t.initialParameter,appliedGradient:i,parameterDelta:k(a-t.initialParameter,`parameter delta`),parameterAfter:a,gradientBufferAfterStep:vn(r,`grad_w_after_step`)}}function hn(e,t,n){let r=new Map([[`x`,[...n.x]],[`residual`,[...n.residual]],[`w`,t.initialParameter],[`grad_w`,t.gradientBufferBefore]]);for(let n of e.instructions)switch(n.op){case`LOAD_SAVED_COLUMN`:r.set(n.output,[...gn(r,n.inputs[0])]);break;case`LOSS_GRAD_COLUMN`:r.set(n.output,[...gn(r,`residual_col`)]);break;case`PARAMETER_LOCAL_GRAD_COLUMN`:{let e=gn(r,`x_col`),t=gn(r,`d_prediction_col`);r.set(n.output,e.map((e,n)=>k(e*t[n],`matrix local d_w ${n}`)));break}case`INPUT_GRAD_COLUMN`:r.set(n.output,gn(r,`d_prediction_col`).map((e,n)=>k(t.initialParameter*e,`matrix d_x ${n}`)));break;case`REDUCE_SUM_GRAD`:r.set(n.output,yn(gn(r,`local_d_w_col`),`matrix batch gradient`));break;case`ACCUMULATE_GRAD_BUFFER`:r.set(n.output,k(_n(r,`grad_w`)+_n(r,`batch_d_w`),`matrix grad buffer accumulation`));break;case`DIVIDE_GRAD`:r.set(n.output,k(_n(r,`grad_w`)/t.divisor,`matrix applied gradient`));break;case`SGD_UPDATE_SCALAR`:r.set(n.output,k(t.initialParameter-t.learningRate*_n(r,`applied_d_w`),`matrix w_next`));break;case`KEEP_GRAD_BUFFER`:r.set(n.output,_n(r,`grad_w`));break;default:throw Error(`unsupported matrix training op: ${n.op}`)}return{columns:{x:gn(r,`x_col`),residual:gn(r,`residual_col`),dPrediction:gn(r,`d_prediction_col`),localDW:gn(r,`local_d_w_col`),dX:gn(r,`d_x_col`)},gradientBufferBefore:t.gradientBufferBefore,batchGradient:_n(r,`batch_d_w`),gradW:_n(r,`grad_w`),appliedGradient:_n(r,`applied_d_w`),parameterAfter:_n(r,`w_next`),gradientBufferAfterStep:_n(r,`grad_w_after_step`)}}function gn(e,t){let n=e.get(t);if(!Array.isArray(n))throw Error(`missing column: ${t}`);return[...n]}function _n(e,t){let n=e.get(t);if(typeof n!=`number`)throw Error(`missing scalar: ${t}`);return n}function vn(e,t){let n=e.get(t);if(n===void 0)throw Error(`missing scalar: ${t}`);return n}function yn(e,t){let n=0;for(let r of e)n=k(n+r,t);return n}function bn(e,t){let n=0;return t.inputs.forEach((r,i)=>{let a=k(e*r-t.targets[i],`audit residual ${i}`);n=k(n+.5*a*a,`audit loss sum`)}),n}function xn(e){let t=0;for(let n=0;n<e[0].length;n+=1)for(let r=0;r<e.length;r+=1)for(let i=r+1;i<e.length;i+=1)t=Math.max(t,Math.abs(e[r][n]-e[i][n]));return k(t,`parity error`)}function Sn(e){let t=an(e),n=dn(t),r=fn(t,n.neuralIrOutputs),i=sn(),a=cn(),o=ln(),s=pn(i,r,t.initialParameter,t.gradientBufferBefore),c=mn(a,t,s.gradW),l=hn(o,t,r),u=k((bn(t.initialParameter+$t,t)-bn(t.initialParameter-$t,t))/(2*$t),`numerical gradient`),d=k(Math.abs(s.batchGradient-u),`gradient error`),f=Math.max(Math.abs(s.batchGradient-l.batchGradient),Math.abs(s.gradW-l.gradW),Math.abs(c.appliedGradient-l.appliedGradient),Math.abs(c.parameterAfter-l.parameterAfter),Math.abs(c.gradientBufferAfterStep-l.gradientBufferAfterStep));return wn({scenario:t,forward:n,savedValues:r,backwardIr:i,optimizerIr:a,matrixTrainingIr:o,backward:s,optimizer:c,matrixTraining:l,gradientAudit:{analytical:s.batchGradient,numerical:u,absoluteError:d},maxPathError:k(f,`training path error`)})}function Cn(e){let t=en.find(t=>t.id===e);if(t===void 0)throw Error(`unknown backward/optimizer lowering scenario: ${e}`);return Sn(t)}function wn(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>wn(e)),e)}function Tn(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(10)).toString()}function En(e){switch(e.op){case`SEED_LOSS_GRAD`:return`start reverse mode at 1`;case`HALF_SQUARED_ERROR_GRAD`:return`residual x loss seed`;case`PROPAGATE_GRAD`:return`pass through subtraction`;case`PARAMETER_LOCAL_GRAD`:return`x x d_prediction`;case`ACCUMULATE_GRAD`:return`add rows in stable order`;case`INPUT_GRAD`:return`w x d_prediction`;case`READ_GRAD_BUFFER`:return`read persistent grad_w`;case`DIVIDE_GRAD`:return`apply explicit divisor`;case`SGD_UPDATE`:case`SGD_UPDATE_SCALAR`:return`w - rate x gradient`;case`KEEP_GRAD_BUFFER`:return`step does not clear`;case`LOAD_SAVED_COLUMN`:return`load ${e.inputs[0]} rows`;case`LOSS_GRAD_COLUMN`:return`reverse loss as a column`;case`PARAMETER_LOCAL_GRAD_COLUMN`:return`one d_w per row`;case`INPUT_GRAD_COLUMN`:return`one d_x per row`;case`REDUCE_SUM_GRAD`:return`row-ascending reduction`;case`ACCUMULATE_GRAD_BUFFER`:return`add batch sum to persistent grad_w`;default:return e.inputs.join(`, `)}}function Dn(e){let t=Object.entries(e.attributes);return t.length===0?`none`:t.map(([e,t])=>`${e}=${Array.isArray(t)?`[${t.join(`, `)}]`:String(t)}`).join(`; `)}function On(e,t){return t===`backward`?e.backwardIr:t===`optimizer`?e.optimizerIr:e.matrixTrainingIr}function kn(e,t){let n={b0:e.backward.dLoss,b1:e.backward.dResidual,b2:e.backward.dPrediction,b3:e.backward.localDW,b4:e.backward.gradW,b5:e.backward.dX},r={o0:e.backward.gradW,o1:e.optimizer.appliedGradient,o2:e.optimizer.parameterAfter,o3:e.optimizer.gradientBufferAfterStep},i={t0:e.matrixTraining.columns.x,t1:e.matrixTraining.columns.residual,t2:e.matrixTraining.columns.dPrediction,t3:e.matrixTraining.columns.localDW,t4:e.matrixTraining.columns.dX,t5:e.matrixTraining.batchGradient,t6:e.matrixTraining.gradW,t7:e.matrixTraining.appliedGradient,t8:e.matrixTraining.parameterAfter,t9:e.matrixTraining.gradientBufferAfterStep},a=t.lane===`backward`?n[t.id]:t.lane===`optimizer`?r[t.id]:i[t.id];return typeof a==`number`?Tn(a):`[${(a??[]).map(Tn).join(`, `)}]`}function An({lane:e,selection:t,setSelection:n,stream:r}){let i=e===`matrix`?`forward-lowering-matrix-lane`:`forward-lowering-instruction-lane`,a=e===`matrix`?`Matrix training IR`:e===`backward`?`Backward IR`:`Optimizer IR`;return(0,E.jsx)(`div`,{className:i,children:r.instructions.map(r=>(0,E.jsxs)(`button`,{"aria-label":`Open ${a} ${r.id}, ${r.op}`,"aria-pressed":t.lane===e&&t.id===r.id,onClick:()=>n({lane:e,id:r.id}),type:`button`,children:[(0,E.jsx)(`small`,{children:r.id}),(0,E.jsx)(`strong`,{children:r.op}),(0,E.jsx)(`code`,{children:r.output}),(0,E.jsx)(`span`,{children:En(r)})]},r.id))})}function jn(){let[e,t]=(0,l.useState)(`one_row_by_hand`),[n,r]=(0,l.useState)({lane:`backward`,id:`b3`}),i=(0,l.useMemo)(()=>Cn(e),[e]),a=On(i,n.lane).instructions.find(e=>e.id===n.id);return(0,E.jsxs)(`main`,{className:`workspace workspace--forward-lowering`,children:[(0,E.jsxs)(`section`,{className:`forward-lowering-stage`,children:[(0,E.jsxs)(`header`,{className:`forward-lowering-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN30 - saved values -> backward -> optimizer`}),(0,E.jsx)(`h2`,{children:`Backward and optimizer lowering map`}),(0,E.jsx)(`p`,{children:`Keep one trainable multiplication fixed while reverse mode becomes an executable schedule and SGD remains a separate state transition.`})]}),(0,E.jsxs)(`span`,{className:`forward-lowering-chip`,children:[i.backwardIr.instructions.length,` backward -> `,i.optimizerIr.instructions.length,` optimizer -> `,i.matrixTrainingIr.instructions.length,` matrix ops`]})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-graph`,"aria-label":`Production forward saved values`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`1 - save`}),(0,E.jsx)(`h2`,{children:`The production forward pass leaves evidence`})]}),(0,E.jsxs)(`code`,{children:[`max forward error `,i.forward.maxError.toExponential(1)]})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-edge-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:`NeuralIR`}),(0,E.jsx)(`span`,{children:i.forward.neuralOps.join(` -> `)}),(0,E.jsx)(`strong`,{children:i.forward.neuralIrOutputs.map(Tn).join(`, `)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:`MatrixIR`}),(0,E.jsx)(`span`,{children:i.forward.matrixOps.join(` -> `)}),(0,E.jsx)(`strong`,{children:i.forward.matrixIrOutputs.map(Tn).join(`, `)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:`saved contract`}),(0,E.jsx)(`span`,{children:`x, w, prediction, residual`}),(0,E.jsx)(`strong`,{children:`backward may read them`})]})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-parity-table`,role:`table`,"aria-label":`Saved forward row values`,children:[(0,E.jsxs)(`div`,{className:`forward-lowering-parity-head`,role:`row`,children:[(0,E.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`x`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`target`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`prediction`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`residual`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`loss`})]}),i.savedValues.x.map((e,t)=>(0,E.jsxs)(`div`,{role:`row`,children:[(0,E.jsx)(`strong`,{role:`cell`,children:t}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(e)}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.savedValues.target[t])}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.savedValues.prediction[t])}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.savedValues.residual[t])}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.savedValues.loss[t])})]},t))]})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`Backward instruction stream`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`2 - reverse`}),(0,E.jsx)(`h2`,{children:`Backward produces gradients`})]}),(0,E.jsxs)(`code`,{children:[i.backwardIr.magic,` v`,i.backwardIr.version]})]}),(0,E.jsx)(An,{lane:`backward`,selection:n,setSelection:r,stream:i.backwardIr})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`Optimizer instruction stream`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`3 - update policy`}),(0,E.jsx)(`h2`,{children:`The optimizer consumes the buffer`})]}),(0,E.jsxs)(`code`,{children:[i.optimizerIr.magic,` v`,i.optimizerIr.version]})]}),(0,E.jsx)(An,{lane:`optimizer`,selection:n,setSelection:r,stream:i.optimizerIr})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`Matrix training operation stream`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`4 - batch`}),(0,E.jsx)(`h2`,{children:`Columns reduce into shared parameter state`})]}),(0,E.jsxs)(`code`,{children:[i.matrixTrainingIr.magic,` v`,i.matrixTrainingIr.version]})]}),(0,E.jsx)(An,{lane:`matrix`,selection:n,setSelection:r,stream:i.matrixTrainingIr})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-selection`,"aria-label":`Selected training lowering detail`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`selected translation`}),(0,E.jsx)(`h2`,{children:a?.op})]}),(0,E.jsx)(`code`,{children:n.id})]}),a===void 0?null:(0,E.jsxs)(`div`,{className:`forward-lowering-detail-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`reads -> writes`}),(0,E.jsxs)(`code`,{children:[a.inputs.join(`, `)||`none`,` -> `,a.output]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`observed value`}),(0,E.jsx)(`code`,{children:kn(i,n)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`attributes`}),(0,E.jsx)(`code`,{children:Dn(a)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`graph provenance`}),(0,E.jsx)(`code`,{children:[...a.sourceNodes,...a.sourceEdges].join(`, `)||`none`})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`lowered from`}),(0,E.jsx)(`code`,{children:a.sourceInstructions.join(`, `)||`direct semantic rule`})]})]})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-parity`,"aria-label":`Backward optimizer execution parity`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`5 - prove equivalence`}),(0,E.jsx)(`h2`,{children:`Scalar and matrix training agree`})]}),(0,E.jsxs)(`code`,{children:[`max error `,i.maxPathError.toExponential(1)]})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-parity-table`,role:`table`,"aria-label":`Backward row gradient values`,children:[(0,E.jsxs)(`div`,{className:`forward-lowering-parity-head`,role:`row`,children:[(0,E.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`x`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`target`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`d prediction`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`local d w`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`d x`})]}),i.backward.dPrediction.map((e,t)=>(0,E.jsxs)(`div`,{role:`row`,children:[(0,E.jsx)(`strong`,{role:`cell`,children:t}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.scenario.inputs[t])}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.scenario.targets[t])}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(e)}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.backward.localDW[t])}),(0,E.jsx)(`code`,{role:`cell`,children:Tn(i.backward.dX[t])})]},t))]}),(0,E.jsxs)(`div`,{className:`forward-lowering-edge-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:`persistent accumulation`}),(0,E.jsxs)(`span`,{children:[Tn(i.backward.gradientBufferBefore),` before + `,i.backward.localDW.map(Tn).join(` + `)]}),(0,E.jsxs)(`strong`,{children:[`grad_w = `,Tn(i.backward.gradW)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:`explicit divisor`}),(0,E.jsxs)(`span`,{children:[Tn(i.backward.gradW),` / `,i.scenario.divisor]}),(0,E.jsxs)(`strong`,{children:[`applied = `,Tn(i.optimizer.appliedGradient)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:`SGD update`}),(0,E.jsxs)(`span`,{children:[Tn(i.optimizer.parameterBefore),` - `,Tn(i.scenario.learningRate),` x `,Tn(i.optimizer.appliedGradient)]}),(0,E.jsxs)(`strong`,{children:[`w_next = `,Tn(i.optimizer.parameterAfter)]})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`forward-lowering-controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Run shape`}),(0,E.jsx)(`h2`,{children:`Keep the programs fixed`}),(0,E.jsx)(`p`,{children:`Change the number of rows or enter with a nonzero buffer while the programs stay fixed.`}),(0,E.jsx)(`div`,{className:`forward-lowering-scenario-buttons`,children:en.map(n=>(0,E.jsxs)(`button`,{"aria-label":n.title,"aria-pressed":e===n.id,onClick:()=>t(n.id),type:`button`,children:[(0,E.jsx)(`strong`,{children:n.title}),(0,E.jsx)(`span`,{children:n.summary})]},n.id))}),(0,E.jsxs)(`div`,{className:`forward-lowering-equation`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Paper result`}),(0,E.jsx)(`code`,{children:`loss = 0.5(w x x - target)^2`}),(0,E.jsx)(`code`,{children:`d_w = (prediction - target) x x`}),(0,E.jsx)(`code`,{children:`grad_w = grad_w_before + sum(d_w)`}),(0,E.jsx)(`code`,{children:`w_next = w - rate x (grad_w / divisor)`})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-mental-model`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Gradient audit`}),(0,E.jsx)(`h2`,{children:`Different route, same slope`}),(0,E.jsxs)(`p`,{children:[`Finite difference `,Tn(i.gradientAudit.numerical),` vs backward `,Tn(i.gradientAudit.analytical),`; error `,i.gradientAudit.absoluteError.toExponential(1),`.`]})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-mental-model`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Rust boundary`}),(0,E.jsx)(`h2`,{children:`Tensor math, explicit policy`}),(0,E.jsx)(`p`,{children:`Rust may accelerate multiply, add, and ReduceSum. The host still owns saved values, divisor, update timing, and zeroing.`})]})]})]})}var Mn={schema_version:1,id:`dense-backend-parity`,title:`One dense column across CPU, Rust, and accelerator boundaries`,question:`What changes, and what must stay equal, when y = XW + B moves to another backend?`,absolute_tolerance:1e-6,graph:{equation:`y = XW + B`,dtype:`f32`,input_shape:[3,1],weight_shape:[1,1],bias_shape:[3,1],output_shape:[3,1],weight:[2],bias:[1,1,1],matrix_ir_file:`../matrix-ir/00-dense-batch.graph.json`},scenario:{id:`three_row_dense`,inputs:[1,2,3],input_payload_file:`../payloads/00-input-x.f32le.hex`,expected_payload_file:`../payloads/00-expected-output.f32le.hex`,expected:{products:[2,4,6],outputs:[3,5,7]}},lanes:[{id:`scalar_cpu`,title:`Scalar CPU reference`,runtime:`NN00 bytecode interpreter`,precision:`binary64`,availability:`required`,steps:[`load one x`,`multiply by w`,`add b`,`store one y`],residency:[`host:x`,`host:product`,`host:y`],expected_outputs:[3,5,7]},{id:`typescript_matrix_cpu`,title:`TypeScript matrix CPU`,runtime:`NN01 CANM matrix plan`,precision:`binary64`,availability:`required`,steps:[`load x column`,`scale column by w`,`broadcast and add b`,`store y column`],residency:[`host:x[3x1]`,`host:product[3x1]`,`host:y[3x1]`],expected_outputs:[3,5,7]},{id:`rust_matrix_cpu`,title:`Rust matrix CPU core`,runtime:`MatrixIR JSON -> matrix-rust-napi -> matrix-cpu`,precision:`f32`,availability:`required-in-native-test`,steps:[`decode MatrixIR JSON`,`MatMul X by W`,`Add broadcast B`,`download output bytes`],residency:[`host:x bytes`,`rust:x,W,B buffers`,`rust:y buffer`,`host:y bytes`],expected_outputs:[3,5,7]},{id:`webgpu_accelerated`,title:`WebGPU accelerator`,runtime:`NN01 async WebGpuMatrixBackend`,precision:`f32`,availability:`optional-runtime-probe`,steps:[`upload x column`,`scale on device`,`add bias on device`,`download y and value trace`],residency:[`host:x`,`device:x,product,bias,y`,`host:output y`,`host:trace x,bias,y`],expected_outputs:[3,5,7]}]},Nn=/^[a-z][a-z0-9_]{0,63}$/,Pn=512,Fn=1e6,In=[`scalar_cpu`,`typescript_matrix_cpu`,`rust_matrix_cpu`,`webgpu_accelerated`];function Ln(e,t,n){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`${n} must be an object`);let r=Object.keys(e).sort(),i=[...t].sort();if(r.join(`,`)!==i.join(`,`))throw Error(`${n} has unexpected fields`);return e}function Rn(e,t){if(typeof e!=`string`||e.length<1||e.length>Pn)throw Error(`${t} must be bounded text`);return e}function zn(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>Fn)throw Error(`${t} must be finite and bounded`);return e}function Bn(e,t,n){if(!Array.isArray(e)||e.length!==t)throw Error(`${n} must contain exactly ${t} numbers`);return e.map((e,t)=>zn(e,`${n}[${t}]`))}function Vn(e,t,n,r){if(!Array.isArray(e)||e.length<t||e.length>n)throw Error(`${r} has an invalid length`);return e.map((e,t)=>Rn(e,`${r}[${t}]`))}function Hn(e){let t=Ln(e,[`schema_version`,`id`,`title`,`question`,`absolute_tolerance`,`graph`,`scenario`,`lanes`],`backend parity fixture`);if(t.schema_version!==1||t.id!==`dense-backend-parity`)throw Error(`backend parity fixture identity is not canonical`);let n=zn(t.absolute_tolerance,`absolute tolerance`);if(n!==1e-6)throw Error(`backend parity tolerance is not canonical`);let r=Ln(t.graph,[`equation`,`dtype`,`input_shape`,`weight_shape`,`bias_shape`,`output_shape`,`weight`,`bias`,`matrix_ir_file`],`backend parity graph`);if(r.equation!==`y = XW + B`||r.dtype!==`f32`||r.matrix_ir_file!==`../matrix-ir/00-dense-batch.graph.json`)throw Error(`backend parity graph contract is not canonical`);let i=Bn(r.weight,1,`graph weight`),a=Bn(r.bias,3,`graph bias`),o={input:Bn(r.input_shape,2,`input shape`),weight:Bn(r.weight_shape,2,`weight shape`),bias:Bn(r.bias_shape,2,`bias shape`),output:Bn(r.output_shape,2,`output shape`)};if(i[0]!==2||a.join(`,`)!==`1,1,1`||o.input.join(`,`)!==`3,1`||o.weight.join(`,`)!==`1,1`||o.bias.join(`,`)!==`3,1`||o.output.join(`,`)!==`3,1`)throw Error(`backend parity dense values and shapes are not canonical`);let s=Ln(t.scenario,[`id`,`inputs`,`input_payload_file`,`expected_payload_file`,`expected`],`backend parity scenario`);if(s.id!==`three_row_dense`||s.input_payload_file!==`../payloads/00-input-x.f32le.hex`||s.expected_payload_file!==`../payloads/00-expected-output.f32le.hex`)throw Error(`backend parity scenario contract is not canonical`);let c=Ln(s.expected,[`products`,`outputs`],`scenario expected`),l=Bn(s.inputs,3,`scenario inputs`),u=Bn(c.products,3,`scenario products`),d=Bn(c.outputs,3,`scenario outputs`);if(l.join(`,`)!==`1,2,3`||u.join(`,`)!==`2,4,6`||d.join(`,`)!==`3,5,7`)throw Error(`backend parity scenario values are not canonical`);if(!Array.isArray(t.lanes)||t.lanes.length!==4)throw Error(`backend parity fixture must contain four lanes`);let f=t.lanes.map((e,t)=>{let n=Ln(e,[`id`,`title`,`runtime`,`precision`,`availability`,`steps`,`residency`,`expected_outputs`],`backend lane ${t}`),r=Rn(n.id,`backend lane ${t} id`);if(!Nn.test(r)||r!==In[t])throw Error(`backend parity lane roster is not canonical`);let i=n.precision;if(i!==`binary64`&&i!==`f32`)throw Error(`backend lane ${t} precision is invalid`);let a=n.availability;if(a!==`required`&&a!==`required-in-native-test`&&a!==`optional-runtime-probe`)throw Error(`backend lane ${t} availability is invalid`);let o=Bn(n.expected_outputs,3,`backend lane ${t} outputs`);if(o.join(`,`)!==d.join(`,`))throw Error(`backend lane ${t} output oracle is dishonest`);return{id:r,title:Rn(n.title,`backend lane ${t} title`),runtime:Rn(n.runtime,`backend lane ${t} runtime`),precision:i,availability:a,steps:Vn(n.steps,4,4,`backend lane ${t} steps`),residency:Vn(n.residency,3,4,`backend lane ${t} residency`),expectedOutputs:o}});return Qn({id:`dense-backend-parity`,title:Rn(t.title,`fixture title`),question:Rn(t.question,`fixture question`),absoluteTolerance:n,graph:{equation:`y = XW + B`,dtype:`f32`,weight:i[0],bias:a,shapes:o},scenario:{id:`three_row_dense`,inputs:l,products:u,outputs:d},lanes:f})}var Un=Hn(Mn);function Wn(e=Un){let t=Qe(`backend-parity-dense`);return $e(t,`x`),et(t,`bias`,e.graph.bias[0]),tt(t,`dense`,[{from:`x`,weight:e.graph.weight,edgeId:`weight`},{from:`bias`,weight:1,edgeId:`bias`}]),rt(t,`output`,`dense`,`y`,{},`dense_to_output`),t}function Gn(){let e=ot(Wn());return{bytecode:e,plan:Ct(e)}}function Kn(e,t){return e.map((e,n)=>{if(!Number.isFinite(e)||Math.abs(e)>Fn)throw Error(`${t}[${n}] is not finite and bounded`);return e})}function qn(e,t){return Math.max(...e.map((e,n)=>Math.abs(e-t[n])))}function Jn(e){return e===`rust_matrix_cpu`?`validated-native-fixture`:e===`webgpu_accelerated`?`deterministic-oracle`:`executed-production`}function Yn(){let e=Un,{bytecode:t,plan:n}=Gn(),r=e.scenario.inputs.map(e=>ct(t,{x:e}).outputs.y),i=Tt(n,{x:e.scenario.inputs}).outputs.y??[],a={scalar_cpu:Kn(r,`scalar outputs`),typescript_matrix_cpu:Kn(i,`matrix outputs`),rust_matrix_cpu:e.scenario.outputs,webgpu_accelerated:e.scenario.outputs.map(e=>Math.fround(e))},o=e.lanes.map(t=>{let n=a[t.id];return{...t,outputs:n,maxAbsoluteError:qn(n,e.scenario.outputs),evidence:Jn(t.id)}});return Qn({fixture:e,products:e.scenario.inputs.map(t=>t*e.graph.weight),scalarInstructionCount:t.functions[0]?.instructions.length??0,matrixOperationCount:n.instructions.length,lanes:o,maxAbsoluteError:Math.max(...o.map(e=>e.maxAbsoluteError))})}async function Xn(e){let{plan:t}=Gn(),n=Kn((await wt(t,{x:Un.scenario.inputs},e)).outputs.y??[],`accelerated outputs`);if(n.length!==Un.scenario.outputs.length)throw Error(`accelerated backend returned the wrong output shape`);let r=qn(n,Un.scenario.outputs),i=r<=Un.absoluteTolerance;return{status:`executed`,outputs:n,maxAbsoluteError:r,withinTolerance:i,message:i?`The async backend executed the production matrix plan and matched the oracle.`:`The async backend executed the production matrix plan but missed the tolerance.`}}async function Zn(){if(!zt.isNavigatorAvailable())return{status:`unavailable`,message:`This browser does not expose WebGPU.`};let e=null;try{return e=await zt.createFromNavigator({powerPreference:`high-performance`}),e===null?{status:`unavailable`,message:`No WebGPU adapter was available.`}:await Xn(e)}catch(e){return{status:`failed`,message:(e instanceof Error?e.message:`WebGPU execution failed`).slice(0,256)}}finally{e?.destroy()}}function Qn(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>Qn(e)),e)}function $n(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(9)).toString()}function er(e){switch(e){case`executed-production`:return`executed here`;case`validated-native-fixture`:return`native fixture proof`;case`deterministic-oracle`:return`oracle until probed`}}function tr(){let e=(0,l.useMemo)(()=>Yn(),[]),[t,n]=(0,l.useState)(`rust_matrix_cpu`),[r,i]=(0,l.useState)({status:`not-run`,message:`Run the probe to ask this browser for a real WebGPU adapter.`}),a=e.lanes.find(e=>e.id===t);async function o(){i({status:`running`,message:`Requesting a WebGPU adapter and executing the plan…`}),i(await Zn())}return(0,E.jsxs)(`main`,{className:`workspace workspace--backend-parity`,children:[(0,E.jsxs)(`section`,{className:`backend-parity-stage`,children:[(0,E.jsxs)(`header`,{className:`backend-parity-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN31 · one graph, four execution engines`}),(0,E.jsx)(`h2`,{children:`Backend parity laboratory`}),(0,E.jsx)(`p`,{children:e.fixture.question})]}),(0,E.jsxs)(`span`,{className:`backend-parity-chip`,children:[`max error `,e.maxAbsoluteError.toExponential(1)]})]}),(0,E.jsxs)(`section`,{className:`backend-parity-paper`,"aria-label":`Dense layer hand calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`1 · calculate`}),(0,E.jsx)(`h2`,{children:`Do the middle row on paper`})]}),(0,E.jsx)(`code`,{children:`y = XW + B`})]}),(0,E.jsxs)(`div`,{className:`backend-parity-equation-flow`,children:[(0,E.jsx)(`code`,{children:`x = 2`}),(0,E.jsx)(`span`,{children:`×`}),(0,E.jsx)(`code`,{children:`w = 2`}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsx)(`code`,{children:`4`}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsx)(`code`,{children:`b = 1`}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsx)(`strong`,{children:`5`})]}),(0,E.jsxs)(`div`,{className:`backend-parity-paper-table`,role:`table`,"aria-label":`Hand calculated dense layer rows`,children:[(0,E.jsxs)(`div`,{className:`backend-parity-table-head`,role:`row`,children:[(0,E.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`x`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`x × 2`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`+ 1`})]}),e.fixture.scenario.inputs.map((t,n)=>(0,E.jsxs)(`div`,{role:`row`,children:[(0,E.jsx)(`strong`,{role:`cell`,children:n}),(0,E.jsx)(`code`,{role:`cell`,children:$n(t)}),(0,E.jsx)(`code`,{role:`cell`,children:$n(e.products[n])}),(0,E.jsx)(`code`,{role:`cell`,children:$n(e.fixture.scenario.outputs[n])})]},n))]})]}),(0,E.jsxs)(`section`,{className:`backend-parity-lanes`,"aria-label":`Backend execution lanes`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`2 · schedule`}),(0,E.jsx)(`h2`,{children:`Same graph, different work plans`})]}),(0,E.jsxs)(`code`,{children:[e.scalarInstructionCount,` scalar · `,e.matrixOperationCount,` matrix`]})]}),(0,E.jsx)(`div`,{className:`backend-parity-lane-grid`,children:e.lanes.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Inspect ${e.title}`,"aria-pressed":t===e.id,onClick:()=>n(e.id),type:`button`,children:[(0,E.jsxs)(`small`,{children:[e.precision,` · `,er(e.evidence)]}),(0,E.jsx)(`strong`,{children:e.title}),(0,E.jsx)(`span`,{children:e.runtime}),(0,E.jsxs)(`code`,{children:[`[`,e.outputs.map($n).join(`, `),`]`]})]},e.id))})]}),(0,E.jsxs)(`section`,{className:`backend-parity-inspector`,"aria-label":`Selected backend detail`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`3 · inspect `,a.precision]}),(0,E.jsx)(`h2`,{children:a.title})]}),(0,E.jsx)(`code`,{children:a.availability})]}),(0,E.jsxs)(`div`,{className:`backend-parity-detail-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`operations`}),(0,E.jsx)(`ol`,{children:a.steps.map(e=>(0,E.jsx)(`li`,{children:e},e))})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`buffer residency`}),(0,E.jsx)(`ol`,{children:a.residency.map(e=>(0,E.jsx)(`li`,{children:(0,E.jsx)(`code`,{children:e})},e))})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`proof`}),(0,E.jsx)(`strong`,{children:er(a.evidence)}),(0,E.jsxs)(`p`,{children:[`maximum absolute error: `,(0,E.jsx)(`code`,{children:a.maxAbsoluteError.toExponential(1)})]})]})]})]}),(0,E.jsxs)(`section`,{className:`backend-parity-results`,"aria-label":`Backend output parity`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`4 · compare`}),(0,E.jsx)(`h2`,{children:`Every lane meets the same oracle`})]}),(0,E.jsxs)(`code`,{children:[`tolerance `,e.fixture.absoluteTolerance]})]}),(0,E.jsxs)(`div`,{className:`backend-parity-results-table`,role:`table`,"aria-label":`CPU Rust and accelerator outputs`,children:[(0,E.jsxs)(`div`,{className:`backend-parity-table-head`,role:`row`,children:[(0,E.jsx)(`strong`,{role:`columnheader`,children:`lane`}),e.fixture.scenario.inputs.map((e,t)=>(0,E.jsxs)(`strong`,{role:`columnheader`,children:[`row `,t]},t)),(0,E.jsx)(`strong`,{role:`columnheader`,children:`error`})]}),e.lanes.map(e=>(0,E.jsxs)(`div`,{role:`row`,children:[(0,E.jsx)(`strong`,{role:`cell`,children:e.title}),e.outputs.map((e,t)=>(0,E.jsx)(`code`,{role:`cell`,children:$n(e)},t)),(0,E.jsx)(`code`,{role:`cell`,children:e.maxAbsoluteError.toExponential(1)})]},e.id))]})]}),(0,E.jsxs)(`section`,{className:`backend-parity-probe`,"aria-label":`WebGPU runtime probe`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`5 · prove the hardware claim`}),(0,E.jsx)(`h2`,{children:`Browser accelerator probe`}),(0,E.jsx)(`p`,{children:r.message}),r.status===`executed`?(0,E.jsxs)(`code`,{children:[`[`,r.outputs.map($n).join(`, `),`] · error `,r.maxAbsoluteError.toExponential(1),` · `,r.withinTolerance?`parity pass`:`parity mismatch`]}):null]}),(0,E.jsxs)(`div`,{className:`backend-parity-probe-status backend-parity-probe-status--${r.status}`,children:[(0,E.jsx)(`strong`,{children:r.status}),(0,E.jsx)(`button`,{disabled:r.status===`running`,onClick:o,type:`button`,children:r.status===`running`?`Running…`:`Run WebGPU probe`})]})]})]}),(0,E.jsxs)(`aside`,{className:`backend-parity-controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Mental model`}),(0,E.jsx)(`h2`,{children:`Meaning above, mechanics below`}),(0,E.jsx)(`p`,{children:`The graph owns the equation. A backend owns scheduling, precision, buffers, and transfers.`}),(0,E.jsxs)(`div`,{className:`backend-parity-rule`,children:[(0,E.jsx)(`code`,{children:`same graph`}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsx)(`code`,{children:`same input`}),(0,E.jsx)(`span`,{children:`→`}),(0,E.jsx)(`strong`,{children:`equal output`})]}),(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Rust boundary`}),(0,E.jsxs)(`p`,{children:[`MatrixIR JSON and little-endian f32 buffers are shared. The Node-free Rust helper test executes the checked-in bytes through `,(0,E.jsx)(`code`,{children:`matrix-cpu`}),`.`]})]}),(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Language direction`}),(0,E.jsx)(`p`,{children:`New language ports can replay this oracle natively, then swap in a Rust binding. A stable C ABI remains an explicit future tranche.`})]}),(0,E.jsxs)(`section`,{className:`backend-parity-warning`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Do not confuse`}),(0,E.jsx)(`p`,{children:`Equal answers prove correctness. They do not prove the GPU is faster—or that it ran at all.`})]})]})]})}var nr=[2,1,3,0,4,2],rr=[1,-1,2],ir=[6,-2,10,0],ar=.02,or=1e-6;function sr(e){return e===0?0:e}function cr(e,t){if(e.length===0||t.length===0)throw Error(`Signal and kernel must contain at least one number.`);if(t.length>e.length)throw Error(`The kernel cannot be longer than the signal in valid mode.`);if(![...e,...t].every(Number.isFinite))throw Error(`Signal and kernel values must be finite numbers.`);return Array.from({length:e.length-t.length+1},(n,r)=>{let i=e.slice(r,r+t.length),a=i.map((e,n)=>sr(e*t[n])),o=a.reduce((e,t)=>[...e,e[e.length-1]+t],[0]);return{outputIndex:r,startIndex:r,window:i,products:a,accumulator:o,output:o[o.length-1]}})}function lr(e,t){if(e.length===0||e.length!==t.length)throw Error(`Outputs and targets must have the same non-zero length.`);return e.reduce((e,n,r)=>e+(n-t[r])**2,0)/e.length}function ur(e,t,n){let r=cr(e,t),i=r.map(e=>e.output);if(n.length!==i.length||!n.every(Number.isFinite))throw Error(`Expected ${i.length} finite target values.`);let a=i.map((e,t)=>sr(e-n[t])),o=a.map(e=>sr(2*e/a.length)),s=t.map(()=>0),c=r.map((e,t)=>{let n=o[t],r=e.window.map((e,t)=>{let r=sr(n*e);return s[t]=sr(s[t]+r),r});return{outputIndex:t,window:e.window,outputGradient:n,kernelGradient:r}});return{outputs:i,errors:a,loss:lr(i,n),outputGradients:o,contributions:c,kernelGradient:s}}function dr(e,t,n,r=or){if(!Number.isFinite(r)||r<=0)throw Error(`Finite-difference epsilon must be positive.`);return t.map((i,a)=>{let o=[...t],s=[...t];o[a]+=r,s[a]-=r;let c=cr(e,o).map(e=>e.output),l=cr(e,s).map(e=>e.output);return(lr(c,n)-lr(l,n))/(2*r)})}function fr(e,t,n,r){if(!Number.isFinite(r)||r<=0)throw Error(`Learning rate must be positive.`);let i=ur(e,t,n),a=t.map((e,t)=>sr(e-r*i.kernelGradient[t])),o=cr(e,a).map(e=>e.output);return{nextKernel:a,nextOutputs:o,nextLoss:lr(o,n)}}function pr(e){let t=e.split(`,`).map(e=>e.trim());if(t.length===0||t.some(e=>e===``))return null;let n=t.map(Number);return n.every(Number.isFinite)?n:null}function mr(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function hr(e){return e.join(`, `)}function gr(){let[e,t]=(0,l.useState)(hr(nr)),[n,r]=(0,l.useState)(hr(rr)),[i,a]=(0,l.useState)(hr(ir)),[o,s]=(0,l.useState)(ar),[c,u]=(0,l.useState)(0),[d,f]=(0,l.useState)(0),p=(0,l.useMemo)(()=>pr(e),[e]),m=(0,l.useMemo)(()=>pr(n),[n]),h=(0,l.useMemo)(()=>pr(i),[i]),g=p===null||m===null?`Use comma-separated finite numbers.`:m.length>p.length?`The kernel must fit entirely inside the signal in valid mode.`:null,_=(0,l.useMemo)(()=>g===null?cr(p,m):[],[g,m,p]),v=g===null?h===null?`Use comma-separated finite training targets.`:h.length===_.length?!Number.isFinite(o)||o<=0?`The learning rate must be a positive number.`:null:`Valid mode produces ${_.length} outputs, so enter ${_.length} targets.`:g,y=(0,l.useMemo)(()=>v===null?ur(p,m,h):null,[m,p,h,v]),b=(0,l.useMemo)(()=>v===null?dr(p,m,h):[],[m,p,h,v]),x=(0,l.useMemo)(()=>v===null?fr(p,m,h,o):null,[m,o,p,h,v]),S=y!==null&&y.kernelGradient.every((e,t)=>Math.abs(e-b[t])<=1e-7);(0,l.useEffect)(()=>{f(e=>Math.min(e,Math.max(_.length-1,0)))},[_.length]);let C=_[d],w=y?.contributions[d],ee=C===void 0?-1:C.startIndex+C.window.length;function te(){t(hr(nr)),r(hr(rr)),a(hr(ir)),s(ar),u(0),f(0)}function T(){x!==null&&(r(hr(x.nextKernel)),u(e=>e+1))}return(0,E.jsxs)(`main`,{className:`workspace workspace--convolution`,children:[(0,E.jsxs)(`section`,{className:`convolution-stage`,"aria-label":`Sliding kernel trace`,children:[(0,E.jsxs)(`div`,{className:`convolution-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN05 · spatial networks`}),(0,E.jsx)(`h2`,{children:`Sliding-kernel microscope`}),(0,E.jsx)(`p`,{children:`One small detector reuses the same weights at every position. Select an output to expose the exact window, products, and running sum that made it.`})]}),(0,E.jsx)(`div`,{className:`convolution-mode-chip`,children:`valid · stride 1 · no flip`})]}),C===void 0||p===null||m===null?(0,E.jsx)(`div`,{className:`convolution-error`,role:`alert`,children:g}):(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`section`,{className:`kernel-slide`,"aria-label":`Kernel over signal`,children:[(0,E.jsxs)(`div`,{className:`array-label`,children:[(0,E.jsx)(`span`,{children:`signal`}),(0,E.jsxs)(`code`,{children:[p.length,` values`]})]}),(0,E.jsx)(`div`,{className:`signal-array`,style:{gridTemplateColumns:`repeat(${p.length}, minmax(48px, 1fr))`},children:p.map((e,t)=>(0,E.jsxs)(`div`,{className:t>=C.startIndex&&t<ee?`signal-cell signal-cell--active`:`signal-cell`,children:[(0,E.jsxs)(`small`,{children:[`x[`,t,`]`]}),(0,E.jsx)(`strong`,{children:mr(e)})]},`${t}-${e}`))}),(0,E.jsxs)(`div`,{className:`array-label array-label--kernel`,children:[(0,E.jsx)(`span`,{children:`shared kernel`}),(0,E.jsxs)(`code`,{children:[`starts at x[`,C.startIndex,`]`]})]}),(0,E.jsx)(`div`,{className:`kernel-track`,style:{gridTemplateColumns:`repeat(${p.length}, minmax(48px, 1fr))`},children:(0,E.jsx)(`div`,{className:`kernel-window`,style:{gridColumn:`${C.startIndex+1} / span ${m.length}`,gridTemplateColumns:`repeat(${m.length}, minmax(48px, 1fr))`},children:m.map((e,t)=>(0,E.jsxs)(`div`,{className:`kernel-cell`,children:[(0,E.jsxs)(`small`,{children:[`k[`,t,`]`]}),(0,E.jsx)(`strong`,{children:mr(e)})]},`${t}-${e}`))})})]}),(0,E.jsxs)(`section`,{className:`mac-panel`,"aria-label":`Multiply accumulate trace`,children:[(0,E.jsxs)(`div`,{className:`mac-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Output y[`,C.outputIndex,`]`]}),(0,E.jsx)(`h2`,{children:`Multiply, then accumulate`})]}),(0,E.jsx)(`strong`,{className:`mac-result`,children:mr(C.output)})]}),(0,E.jsx)(`div`,{className:`product-grid`,children:C.products.map((e,t)=>(0,E.jsxs)(`div`,{className:`product-card`,children:[(0,E.jsxs)(`small`,{children:[`term `,t+1]}),(0,E.jsxs)(`code`,{children:[mr(C.window[t]),` × `,mr(m[t])]}),(0,E.jsx)(`strong`,{children:mr(e)})]},t))}),(0,E.jsx)(`div`,{className:`accumulator-strip`,"aria-label":`Running accumulator`,children:C.accumulator.map((e,t)=>(0,E.jsxs)(`div`,{className:`accumulator-step`,children:[(0,E.jsx)(`small`,{children:t===0?`start`:`after term ${t}`}),(0,E.jsx)(`strong`,{children:mr(e)})]},t))}),(0,E.jsxs)(`code`,{className:`expanded-equation`,children:[C.window.map((e,t)=>`${mr(e)}×${mr(m[t])}`).join(` + `),` = `,mr(C.output)]})]}),(0,E.jsxs)(`section`,{className:`output-strip`,"aria-label":`Feature map outputs`,children:[(0,E.jsxs)(`div`,{className:`array-label`,children:[(0,E.jsx)(`span`,{children:`feature map`}),(0,E.jsxs)(`code`,{children:[p.length,` - `,m.length,` + 1 = `,_.length]})]}),(0,E.jsx)(`div`,{className:`output-buttons`,children:_.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Select output ${e.outputIndex}`,className:e.outputIndex===d?`output-button output-button--active`:`output-button`,type:`button`,onClick:()=>f(e.outputIndex),children:[(0,E.jsxs)(`small`,{children:[`y[`,e.outputIndex,`]`]}),(0,E.jsx)(`strong`,{children:mr(e.output)})]},e.outputIndex))})]}),(0,E.jsxs)(`section`,{className:`training-panel`,"aria-label":`Shared kernel gradient trace`,children:[(0,E.jsxs)(`div`,{className:`training-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN06 · backward pass`}),(0,E.jsx)(`h2`,{children:`Shared weights collect gradients`}),(0,E.jsx)(`p`,{children:`Every output sends a contribution back to each kernel weight. Columns add because the same weight was reused in every window.`})]}),(0,E.jsxs)(`div`,{className:S?`gradient-check-badge gradient-check-badge--pass`:`gradient-check-badge`,children:[(0,E.jsx)(`small`,{children:`finite difference`}),(0,E.jsx)(`strong`,{children:S?`PASS`:`CHECK`})]})]}),y===null||x===null||w===void 0?(0,E.jsx)(`div`,{className:`convolution-error`,role:`alert`,children:v}):(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`div`,{className:`loss-flow`,"aria-label":`Loss before and after proposed step`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`current MSE`}),(0,E.jsx)(`strong`,{children:mr(y.loss)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`− η∇`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`after proposed step`}),(0,E.jsx)(`strong`,{children:mr(x.nextLoss)})]})]}),(0,E.jsxs)(`section`,{className:`selected-gradient-path`,"aria-label":`Selected output gradient path`,children:[(0,E.jsxs)(`div`,{className:`mac-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected path · y[`,d,`]`]}),(0,E.jsx)(`h3`,{children:`One output sends three contributions`})]}),(0,E.jsxs)(`code`,{children:[`dL/dy = 2/`,y.outputs.length,` × `,mr(y.errors[d]),` = `,mr(w.outputGradient)]})]}),(0,E.jsx)(`div`,{className:`product-grid`,children:w.kernelGradient.map((e,t)=>(0,E.jsxs)(`div`,{className:`product-card`,children:[(0,E.jsxs)(`small`,{children:[`toward k[`,t,`]`]}),(0,E.jsxs)(`code`,{children:[mr(w.outputGradient),` × `,mr(w.window[t])]}),(0,E.jsx)(`strong`,{children:mr(e)})]},t))})]}),(0,E.jsx)(`div`,{className:`gradient-table-wrap`,children:(0,E.jsxs)(`table`,{className:`gradient-table`,children:[(0,E.jsx)(`caption`,{children:`Gradient contributions from every reused position`}),(0,E.jsx)(`thead`,{children:(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`col`,children:`weight`}),y.contributions.map(e=>(0,E.jsxs)(`th`,{scope:`col`,children:[`y[`,e.outputIndex,`]`]},e.outputIndex)),(0,E.jsx)(`th`,{scope:`col`,children:`sum`}),(0,E.jsx)(`th`,{scope:`col`,children:`numeric`})]})}),(0,E.jsx)(`tbody`,{children:m.map((e,t)=>(0,E.jsxs)(`tr`,{children:[(0,E.jsxs)(`th`,{scope:`row`,children:[`dL/dk[`,t,`]`]}),y.contributions.map(e=>(0,E.jsx)(`td`,{children:mr(e.kernelGradient[t])},e.outputIndex)),(0,E.jsx)(`td`,{className:`gradient-sum`,children:mr(y.kernelGradient[t])}),(0,E.jsx)(`td`,{children:mr(b[t])})]},t))})]})}),(0,E.jsx)(`div`,{className:`kernel-update-grid`,"aria-label":`Proposed kernel update`,children:m.map((e,t)=>(0,E.jsxs)(`div`,{className:`kernel-update`,children:[(0,E.jsxs)(`small`,{children:[`update k[`,t,`]`]}),(0,E.jsxs)(`code`,{children:[mr(e),` − `,mr(o),` × `,mr(y.kernelGradient[t])]}),(0,E.jsx)(`strong`,{children:mr(x.nextKernel[t])})]},t))})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`convolution-controls`,"aria-label":`Convolution controls`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Change the arithmetic`}),(0,E.jsx)(`h2`,{children:`Signal and detector`}),(0,E.jsx)(`p`,{children:`Use an asymmetric kernel: reversing it should change the outputs.`})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Input signal`}),(0,E.jsx)(`input`,{"aria-label":`Input signal`,value:e,onChange:e=>t(e.target.value)})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Kernel weights`}),(0,E.jsx)(`input`,{"aria-label":`Kernel weights`,value:n,onChange:e=>r(e.target.value)})]}),(0,E.jsxs)(`div`,{className:`convolution-training-controls`,children:[(0,E.jsxs)(`div`,{className:`history__topline`,children:[(0,E.jsx)(`span`,{children:`Train shared weights`}),(0,E.jsxs)(`strong`,{children:[`step `,c]})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Training targets`}),(0,E.jsx)(`input`,{"aria-label":`Training targets`,value:i,onChange:e=>a(e.target.value)})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Learning rate`}),(0,E.jsx)(`input`,{"aria-label":`Convolution learning rate`,min:`0.0001`,step:`0.001`,type:`number`,value:o,onChange:e=>s(Number(e.target.value))})]}),(0,E.jsx)(`button`,{className:`training-step-button`,disabled:x===null,type:`button`,onClick:T,children:`Apply gradient step`})]}),(0,E.jsxs)(`div`,{className:`button-grid`,children:[(0,E.jsx)(`button`,{type:`button`,disabled:d===0,onClick:()=>f(e=>Math.max(e-1,0)),children:`Previous`}),(0,E.jsx)(`button`,{type:`button`,disabled:d>=_.length-1,onClick:()=>f(e=>Math.min(e+1,_.length-1)),children:`Next`}),(0,E.jsx)(`button`,{type:`button`,onClick:te,children:`Reset fixture`})]}),(0,E.jsxs)(`div`,{className:`convolution-note`,children:[(0,E.jsx)(`span`,{children:`Why “no flip”?`}),(0,E.jsx)(`p`,{children:`Neural libraries usually say convolution while computing cross-correlation. Kernel k[0] multiplies the leftmost value in every window. The NN05 fixture makes this convention testable across languages.`})]}),(0,E.jsxs)(`div`,{className:`convolution-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Images add a second spatial direction; channels and batches add more indexed loops. The same shared-gradient reduction still happens for every trainable filter.`})]})]})]})}var _r=[{id:`small-tanh`,label:`Small tanh`,summary:`Weights and tanh derivatives shrink the chain`,input:1,weights:[.5,.5,.5,.5],activation:`tanh`,target:0},{id:`saturated-tanh`,label:`Saturated tanh`,summary:`Large preactivations make tanh derivatives tiny`,input:1,weights:[3,3,3,3],activation:`tanh`,target:0},{id:`unit-relu`,label:`Unit ReLU`,summary:`Local Jacobians stay at one`,input:1,weights:[1,1,1,1],activation:`relu`,target:0},{id:`large-relu`,label:`Large ReLU`,summary:`Every layer doubles the forward and backward signal`,input:1,weights:[2,2,2,2],activation:`relu`,target:0}];function vr(e){let t=_r.find(t=>t.id===e);if(!t)throw Error(`NN24 unknown gradient scenario: ${e}`);return t}function yr(e,t){return t===`tanh`?Math.tanh(e):Math.max(0,e)}function br(e,t,n){return n===`tanh`?1-t**2:+(e>0)}function xr(e,t){return .5*(e.weights.reduce((t,n)=>yr(n*t,e.activation),t)-e.target)**2}function Sr(e=`small-tanh`,t=1e-6){let n=vr(e);if(!Number.isFinite(t)||t<=0)throw Error(`NN24 finite-difference epsilon must be positive and finite.`);if(!Number.isFinite(n.input)||!Number.isFinite(n.target)||n.weights.length<2||!n.weights.every(Number.isFinite))throw Error(`NN24 scenarios need finite values and at least two weights.`);let r=n.input,i=n.weights.map((e,t)=>{let i=e*r,a=yr(i,n.activation),o=br(i,a,n.activation),s={layer:t+1,input:r,weight:e,preactivation:i,activation:a,activationDerivative:o,localJacobian:e*o,upstreamGradient:0,preactivationGradient:0,weightGradient:0,inputGradient:0};return r=a,s}),a=r,o=a-n.target,s=.5*o**2,c=o;for(let e=i.length-1;e>=0;--e){let t=i[e],n=c*t.activationDerivative;t.upstreamGradient=c,t.preactivationGradient=n,t.weightGradient=n*t.input,t.inputGradient=n*t.weight,c=t.inputGradient}let l=i.reduce((e,t)=>e*t.localJacobian,1),u=(xr(n,n.input+t)-xr(n,n.input-t))/(2*t),d=Math.abs(l),f=d<.1?`vanishing`:d>10?`exploding`:`stable`;return{scenario:{...n,weights:[...n.weights]},output:a,outputError:o,loss:s,chainJacobian:l,inputGradient:c,finiteDifferenceInputGradient:u,finiteDifferenceError:Math.abs(c-u),classification:f,layers:i}}function A(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function Cr(){let[e,t]=(0,l.useState)(`small-tanh`),[n,r]=(0,l.useState)(3),i=(0,l.useMemo)(()=>Sr(e),[e]),a=(0,l.useMemo)(()=>_r.map(e=>Sr(e.id)),[]),o=i.layers[n],s=Math.max(...a.map(e=>Math.log10(1+Math.abs(e.inputGradient))),1e-12);return(0,E.jsxs)(`main`,{className:`workspace workspace--gradient-flow`,children:[(0,E.jsxs)(`section`,{className:`gradient-flow-stage`,"aria-label":`Vanishing and exploding gradient explorer`,children:[(0,E.jsxs)(`div`,{className:`gradient-flow-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN24 / reverse one scalar chain`}),(0,E.jsx)(`h2`,{children:`Vanishing and exploding gradients`}),(0,E.jsx)(`p`,{children:`Multiply four local Jacobians and watch one loss gradient travel from the output back to the input.`})]}),(0,E.jsx)(`div`,{className:`gradient-flow-chip gradient-flow-chip--${i.classification}`,children:i.classification})]}),(0,E.jsxs)(`section`,{className:`gradient-forward-panel`,"aria-label":`Gradient flow forward pass`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Forward / save every value`}),(0,E.jsx)(`h2`,{children:`Input to loss`})]}),(0,E.jsxs)(`span`,{children:[`half squared error target `,A(i.scenario.target)]})]}),(0,E.jsxs)(`div`,{className:`gradient-forward-lane`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:`input`}),(0,E.jsx)(`strong`,{children:A(i.scenario.input)})]}),i.layers.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`span`,{children:[`layer `,e.layer]}),(0,E.jsxs)(`code`,{children:[A(e.input),` x `,A(e.weight)]}),(0,E.jsxs)(`strong`,{children:[i.scenario.activation,` = `,A(e.activation)]})]},e.layer)),(0,E.jsxs)(`div`,{className:`gradient-loss-node`,children:[(0,E.jsx)(`span`,{children:`loss`}),(0,E.jsx)(`strong`,{children:A(i.loss)})]})]})]}),(0,E.jsxs)(`section`,{className:`gradient-backward-panel`,"aria-label":`Gradient flow backward pass`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Backward / multiply local slopes`}),(0,E.jsx)(`h2`,{children:`Loss to input`})]}),(0,E.jsxs)(`span`,{children:[`start dL/da4 = `,A(i.outputError)]})]}),(0,E.jsxs)(`div`,{className:`gradient-backward-lane`,children:[[...i.layers].reverse().map(e=>(0,E.jsxs)(`button`,{"aria-pressed":n===e.layer-1,type:`button`,onClick:()=>r(e.layer-1),children:[(0,E.jsxs)(`span`,{children:[`layer `,e.layer]}),(0,E.jsxs)(`small`,{children:[`upstream `,A(e.upstreamGradient)]}),(0,E.jsxs)(`strong`,{children:[`local x `,A(e.localJacobian)]}),(0,E.jsxs)(`code`,{children:[`to input `,A(e.inputGradient)]})]},e.layer)),(0,E.jsxs)(`div`,{className:`gradient-input-node`,children:[(0,E.jsx)(`span`,{children:`input gradient`}),(0,E.jsx)(`strong`,{children:A(i.inputGradient)})]})]})]}),(0,E.jsxs)(`section`,{className:`gradient-arithmetic-panel`,"aria-label":`Selected gradient calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Open layer `,o.layer]}),(0,E.jsx)(`h2`,{children:`One chain-rule step`})]}),(0,E.jsxs)(`span`,{children:[`saved input `,A(o.input)]})]}),(0,E.jsxs)(`div`,{className:`gradient-equation-grid`,children:[(0,E.jsxs)(`code`,{children:[A(o.upstreamGradient),` x `,A(o.activationDerivative),` = `,A(o.preactivationGradient)]}),(0,E.jsx)(`span`,{children:`dL/da x da/dz = dL/dz`}),(0,E.jsxs)(`code`,{children:[A(o.preactivationGradient),` x `,A(o.weight),` = `,A(o.inputGradient)]}),(0,E.jsx)(`span`,{children:`dL/dz x dz/dinput`}),(0,E.jsxs)(`code`,{children:[A(o.preactivationGradient),` x `,A(o.input),` = `,A(o.weightGradient)]}),(0,E.jsx)(`span`,{children:`dL/dz x saved input = dL/dw`})]})]}),(0,E.jsxs)(`section`,{className:`gradient-chain-panel`,"aria-label":`Gradient chain product`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Separate the path from the loss`}),(0,E.jsx)(`h2`,{children:`Total local Jacobian product`})]}),(0,E.jsx)(`strong`,{children:A(i.chainJacobian)})]}),(0,E.jsxs)(`div`,{className:`gradient-chain-equation`,children:[i.layers.map(e=>(0,E.jsx)(`code`,{children:A(e.localJacobian)},e.layer)),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsx)(`strong`,{children:A(i.chainJacobian)})]}),(0,E.jsxs)(`p`,{children:[A(i.outputError),` output error x `,A(i.chainJacobian),` chain = `,(0,E.jsx)(`strong`,{children:A(i.inputGradient)}),` input gradient.`]}),(0,E.jsxs)(`div`,{className:`gradient-audit`,children:[(0,E.jsx)(`span`,{children:`central finite difference`}),(0,E.jsx)(`code`,{children:A(i.finiteDifferenceInputGradient)}),(0,E.jsx)(`span`,{children:`absolute error`}),(0,E.jsx)(`code`,{children:A(i.finiteDifferenceError)})]})]}),(0,E.jsxs)(`section`,{className:`gradient-comparison-panel`,"aria-label":`Gradient scenario comparison`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Four mechanisms side by side`}),(0,E.jsx)(`h2`,{children:`Compare input-gradient magnitude`})]}),(0,E.jsx)(`span`,{children:`bar uses log10(1 + |gradient|)`})]}),(0,E.jsx)(`div`,{className:`gradient-comparison-grid`,children:a.map(t=>(0,E.jsxs)(`article`,{className:t.scenario.id===e?`is-selected`:``,children:[(0,E.jsx)(`strong`,{children:t.scenario.label}),(0,E.jsx)(`span`,{children:t.classification}),(0,E.jsx)(`i`,{style:{width:`${Math.log10(1+Math.abs(t.inputGradient))/s*100}%`}}),(0,E.jsxs)(`code`,{children:[`dL/dinput `,A(t.inputGradient)]}),(0,E.jsxs)(`small`,{children:[`chain `,A(t.chainJacobian)]})]},t.scenario.id))})]})]}),(0,E.jsxs)(`aside`,{className:`controls gradient-flow-controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Gradient mechanism`}),(0,E.jsx)(`h2`,{children:`Choose a chain`}),(0,E.jsx)(`p`,{children:`Each scenario keeps four scalar layers and target zero.`}),(0,E.jsx)(`div`,{className:`gradient-scenario-buttons`,children:_r.map(n=>(0,E.jsxs)(`button`,{"aria-pressed":n.id===e,type:`button`,onClick:()=>t(n.id),children:[(0,E.jsx)(`strong`,{children:n.label}),(0,E.jsx)(`span`,{children:n.summary}),(0,E.jsxs)(`code`,{children:[n.weights.join(` x `),` / `,n.activation]})]},n.id))}),(0,E.jsxs)(`div`,{className:`gradient-flow-reading`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`What to notice`}),(0,E.jsx)(`h2`,{children:i.classification===`vanishing`?`Early layers hear a whisper`:i.classification===`exploding`?`Early layers receive a blast`:`The gradient keeps its scale`}),(0,E.jsx)(`p`,{children:`Changing a weight changes both the forward activation and the local factor used on the reverse path.`})]})]})]})}var wr=[`tiny`,`xavier`,`he`,`large`],Tr=[[1,0],[0,1],[-1,0],[0,-1]],Er=[[[1,-1],[1,1]],[[1,-1],[1,1]],[[1,-1],[1,1]]];function Dr(e,t){if(!Number.isInteger(t)||t<1)throw Error(`NN23 fan-in must be a positive integer.`);return e===`tiny`?.1:e===`xavier`?Math.sqrt(1/t):e===`he`?Math.sqrt(2/t):2}function Or(e,t){let n=e.flat();if(n.length===0||!n.every(Number.isFinite))throw Error(`NN23 distributions need at least one finite value.`);let r=n.reduce((e,t)=>e+t,0)/n.length,i=n.reduce((e,t)=>e+(t-r)**2,0)/n.length;return{mean:r,variance:i,standardDeviation:Math.sqrt(i),minimum:Math.min(...n),maximum:Math.max(...n),zeroFraction:n.filter(e=>Math.abs(e)<1e-12).length/n.length,saturatedFraction:t===`tanh`?n.filter(e=>Math.abs(e)>=.95).length/n.length:0}}function kr(e,t){return t===`tanh`?Math.tanh(e):Math.max(0,e)}function Ar(e=`xavier`,t=`tanh`,n=Tr,r=Er){if(!wr.includes(e))throw Error(`NN23 initializer is not supported.`);if(t!==`tanh`&&t!==`relu`)throw Error(`NN23 activation must be tanh or ReLU.`);if(n.length<2||n[0].length<1)throw Error(`NN23 needs at least two non-empty input rows.`);let i=n[0].length;if(n.some(e=>e.length!==i||!e.every(Number.isFinite)))throw Error(`NN23 inputs must be a finite rectangular matrix.`);if(r.length<1)throw Error(`NN23 needs at least one weight template.`);let a=n.map(e=>[...e]),o=r.map((n,r)=>{let i=a[0].length;if(n.length!==i||n.length===0)throw Error(`NN23 layer ${r+1} template must match fan-in.`);let o=n[0].length;if(o<1||n.some(e=>e.length!==o||!e.every(Number.isFinite)))throw Error(`NN23 layer ${r+1} template must be finite and rectangular.`);let s=Dr(e,i),c=n.map(e=>e.map(e=>e*s)),l=a.map(e=>Array.from({length:o},(t,n)=>e.reduce((e,t,r)=>e+t*c[r][n],0))),u=l.map(e=>e.map(e=>kr(e,t))),d={layer:r+1,fanIn:i,scale:s,weights:c,inputs:a,preactivations:l,activations:u,summary:Or(u,t)};return a=u,d});return{initializer:e,activation:t,inputSummary:Or(n,t),layers:o}}var jr=[{kind:`tiny`,label:`Tiny`,summary:`fixed scale 0.1`},{kind:`xavier`,label:`Xavier`,summary:`sqrt(1 / fan-in)`},{kind:`he`,label:`He`,summary:`sqrt(2 / fan-in)`},{kind:`large`,label:`Large`,summary:`fixed scale 2`}];function Mr(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function Nr(e,t,n){let r=Math.max(n-t,1e-12);return`${(e-t)/r*100}%`}function Pr(){let[e,t]=(0,l.useState)(`xavier`),[n,r]=(0,l.useState)(`tanh`),[i,a]=(0,l.useState)(0),o=(0,l.useMemo)(()=>Ar(e,n),[n,e]),s=(0,l.useMemo)(()=>wr.map(e=>Ar(e,n)),[n]),c=o.layers[i],u=c.inputs[0],d=u.map((e,t)=>e*c.weights[t][0]),f=Math.max(...o.layers.flatMap(e=>e.activations.flat().map(Math.abs)),1),p=Math.max(...s.flatMap(e=>e.layers.map(e=>e.summary.standardDeviation)),1e-12);return(0,E.jsxs)(`main`,{className:`workspace workspace--initialization`,children:[(0,E.jsxs)(`section`,{className:`initialization-stage`,"aria-label":`Initialization distribution explorer`,children:[(0,E.jsxs)(`div`,{className:`lab-intro initialization-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN23 / same signs, different scale`}),(0,E.jsx)(`h2`,{children:`Initialization and activation distributions`}),(0,E.jsx)(`p`,{children:`Follow four tiny inputs through three layers and see when signals shrink, spread, saturate, or explode.`})]}),(0,E.jsxs)(`div`,{className:`initialization-chip`,children:[e,` + `,n]})]}),(0,E.jsxs)(`section`,{className:`initialization-flow`,"aria-label":`Layer activation distributions`,children:[(0,E.jsxs)(`div`,{className:`distribution-card distribution-card--input`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Input batch`}),(0,E.jsx)(`strong`,{children:`4 rows x 2 values`}),(0,E.jsxs)(`code`,{children:[`std `,Mr(o.inputSummary.standardDeviation)]})]}),o.layers.map((e,t)=>{let n=e.activations.flat();return(0,E.jsxs)(`button`,{"aria-pressed":i===t,className:`distribution-card`,type:`button`,onClick:()=>a(t),children:[(0,E.jsxs)(`span`,{className:`eyebrow`,children:[`Layer `,e.layer]}),(0,E.jsxs)(`strong`,{children:[`std `,Mr(e.summary.standardDeviation)]}),(0,E.jsx)(`span`,{className:`distribution-dot-plot`,"aria-hidden":`true`,children:n.map((e,t)=>(0,E.jsx)(`i`,{style:{left:Nr(e,-f,f)}},t))}),(0,E.jsxs)(`span`,{children:[Mr(e.summary.minimum),` to `,Mr(e.summary.maximum)]})]},e.layer)})]}),(0,E.jsxs)(`section`,{className:`distribution-summary-panel`,"aria-label":`Selected activation distribution`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`All eight activations`}),(0,E.jsxs)(`h2`,{children:[`Layer `,c.layer,` distribution`]})]}),(0,E.jsxs)(`span`,{children:[`scale `,Mr(c.scale)]})]}),(0,E.jsxs)(`div`,{className:`distribution-stat-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:`mean`}),(0,E.jsx)(`strong`,{children:Mr(c.summary.mean)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:`variance`}),(0,E.jsx)(`strong`,{children:Mr(c.summary.variance)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:`standard deviation`}),(0,E.jsx)(`strong`,{children:Mr(c.summary.standardDeviation)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:n===`tanh`?`saturated`:`exact zeros`}),(0,E.jsxs)(`strong`,{children:[Mr((n===`tanh`?c.summary.saturatedFraction:c.summary.zeroFraction)*100,3),`%`]})]})]}),(0,E.jsx)(`div`,{className:`activation-value-grid`,children:c.activations.flat().map((e,t)=>(0,E.jsx)(`code`,{children:Mr(e)},t))})]}),(0,E.jsxs)(`section`,{className:`initialization-arithmetic`,"aria-label":`Selected layer hand calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Sample 0 / neuron 0`}),(0,E.jsx)(`h2`,{children:`Open one activation`})]}),(0,E.jsx)(`span`,{children:`no bias in this controlled experiment`})]}),(0,E.jsxs)(`div`,{className:`initialization-equation`,children:[d.map((e,t)=>(0,E.jsxs)(`code`,{children:[Mr(u[t]),` x `,Mr(c.weights[t][0]),` = `,Mr(e)]},t)),(0,E.jsxs)(`strong`,{children:[`sum = `,Mr(c.preactivations[0][0])]}),(0,E.jsxs)(`strong`,{children:[n,` = `,Mr(c.activations[0][0])]})]})]}),(0,E.jsxs)(`section`,{className:`initializer-comparison`,"aria-label":`Initializer comparison`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same inputs / same signs / same activation`}),(0,E.jsx)(`h2`,{children:`Compare signal spread`})]}),(0,E.jsx)(`span`,{children:`bar length = layer standard deviation`})]}),(0,E.jsx)(`div`,{className:`initializer-comparison-grid`,children:s.map(t=>(0,E.jsxs)(`article`,{className:t.initializer===e?`is-selected`:``,children:[(0,E.jsx)(`strong`,{children:t.initializer}),t.layers.map(e=>(0,E.jsxs)(`div`,{className:`spread-row`,children:[(0,E.jsxs)(`span`,{children:[`L`,e.layer]}),(0,E.jsx)(`i`,{style:{width:`${e.summary.standardDeviation/p*100}%`}}),(0,E.jsx)(`code`,{children:Mr(e.summary.standardDeviation)})]},e.layer))]},t.initializer))})]})]}),(0,E.jsxs)(`aside`,{className:`controls initialization-controls`,children:[(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Weight scale`}),(0,E.jsx)(`h2`,{children:`Choose an initializer`}),(0,E.jsx)(`p`,{children:`The sign template stays fixed so only the scaling rule changes.`}),(0,E.jsx)(`div`,{className:`initializer-buttons`,children:jr.map(n=>(0,E.jsxs)(`button`,{"aria-pressed":e===n.kind,type:`button`,onClick:()=>t(n.kind),children:[(0,E.jsx)(`span`,{children:n.label}),(0,E.jsx)(`small`,{children:n.summary})]},n.kind))})]}),(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Nonlinearity`}),(0,E.jsx)(`h2`,{children:`Switch the activation`}),(0,E.jsx)(`div`,{className:`activation-choice-grid`,children:[`tanh`,`relu`].map(e=>(0,E.jsx)(`button`,{"aria-pressed":n===e,type:`button`,onClick:()=>r(e),children:e===`tanh`?`tanh`:`ReLU`},e))})]}),(0,E.jsxs)(`section`,{className:`initialization-reading`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`What to notice`}),(0,E.jsx)(`h2`,{children:e===`tiny`?`Signal is fading`:e===`large`?n===`tanh`?`tanh is pinned near its limits`:`Signal is growing`:`Scale and activation cooperate`}),(0,E.jsx)(`p`,{children:`Real initializers draw random weights. NN23 fixes the signs so every language can reproduce the arithmetic exactly.`})]})]})]})}var Fr=[{id:`plain`,label:`Plain branch`,summary:`The learned branch is the control`},{id:`normalization`,label:`Layer normalization`,summary:`Coordinates share mean and variance`},{id:`dropout`,label:`Inverted dropout`,summary:`A pinned training mask drops and rescales`},{id:`residual`,label:`Identity residual`,summary:`A short skip bypasses the learned branch`}],Ir=[1,1,3,3],Lr=[1,0,0,-1],Rr=[1,0,1,0];function zr(e){return Math.abs(e)<1e-12?0:e}function Br(e,t){return zr(e.reduce((e,n,r)=>e+n*t[r],0))}function Vr(e,t,n){let r=e.reduce((e,t)=>e+t,0)/e.length,i=e.map(e=>zr(e-r)),a=i.reduce((e,t)=>e+t**2,0)/e.length,o=Math.sqrt(a+n);if(o===0)throw Error(`NN25 normalization variance must be positive.`);let s=i.map(e=>zr(e/o));return{mean:r,centered:i,variance:a,standardDeviation:o,normalized:s,upstreamSum:zr(t.reduce((e,t)=>e+t,0)),upstreamDotNormalized:Br(t,s)}}function Hr(e,t,n,r,i,a){let o=t.map(e=>zr(n*e));return e===`plain`?o:e===`normalization`?Vr(o,[0,0,0,0],a).normalized:e===`dropout`?o.map((e,t)=>zr(e*r[t]/i)):o.map((e,n)=>zr(e+t[n]))}function Ur(e,t,n,r,i,a,o,s,c){let l=Hr(e,t,n,i,a,o),u;if(e===`normalization`){let e=t.length,n=e*c.standardDeviation;u=r.map((t,r)=>zr((e*t-c.upstreamSum-c.normalized[r]*c.upstreamDotNormalized)/n))}else u=e===`dropout`?r.map((e,t)=>zr(e*i[t]/a)):[...r];let d=e===`residual`?[...r]:t.map(()=>0),f=u.map((e,t)=>zr(n*e+d[t])),p=Br(u,t),m=Br(r,l),h=t.map((c,l)=>{let u=[...t],d=[...t];return u[l]=u[l]+s,d[l]=d[l]-s,(Br(r,Hr(e,u,n,i,a,o))-Br(r,Hr(e,d,n,i,a,o)))/(2*s)}),g=(Br(r,Hr(e,t,n+s,i,a,o))-Br(r,Hr(e,t,n-s,i,a,o)))/(2*s);return{id:e,output:l,score:m,branchGradient:u,skipGradient:d,inputGradient:f,weightGradient:p,finiteDifferenceInputGradient:h,finiteDifferenceWeightGradient:g,inputGradientAbsoluteError:f.map((e,t)=>Math.abs(e-h[t])),weightGradientAbsoluteError:Math.abs(p-g)}}function Wr(e=Ir,t=.5,n=Lr,r=Rr,i=.5,a=0,o=1e-6){if(e.length!==4||n.length!==4||r.length!==4||!e.every(Number.isFinite)||!n.every(Number.isFinite)||!r.every(e=>e===0||e===1)||!Number.isFinite(t)||!Number.isFinite(i)||i<=0||i>1||!Number.isFinite(a)||a<0||!Number.isFinite(o)||o<=0)throw Error(`NN25 needs four finite coordinates, a binary mask, valid probability, and valid epsilon values.`);let s=e.map(e=>zr(t*e)),c=Vr(s,n,a),l={scaledMask:r.map(e=>zr(e/i)),evaluationOutput:[...s],trainingExpectation:[...s]},u=Fr.map(s=>Ur(s.id,e,t,n,r,i,a,o,c));return{input:[...e],branchWeight:t,upstreamGradient:[...n],dropoutMask:[...r],keepProbability:i,branch:s,normalization:c,dropout:l,routes:u}}function j(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function Gr(e){return`[${e.map(e=>j(e)).join(`, `)}]`}function Kr({label:e,values:t,selectedCoordinate:n,tone:r=`blue`}){return(0,E.jsxs)(`div`,{className:`stabilizer-vector stabilizer-vector--${r}`,children:[(0,E.jsx)(`span`,{children:e}),(0,E.jsx)(`div`,{children:t.map((t,r)=>(0,E.jsxs)(`code`,{className:n===r?`is-selected`:``,children:[(0,E.jsx)(`small`,{children:r+1}),j(t)]},`${e}-${r}`))})]})}function qr(){let[e,t]=(0,l.useState)(`plain`),[n,r]=(0,l.useState)(0),i=(0,l.useMemo)(()=>Wr(),[]),a=i.routes.find(t=>t.id===e),o=Fr.find(t=>t.id===e),s=n,c=Math.max(...a.inputGradientAbsoluteError);return(0,E.jsxs)(`main`,{className:`workspace workspace--stabilizers`,children:[(0,E.jsxs)(`section`,{className:`stabilizer-stage`,"aria-label":`Normalization dropout and residual comparison`,children:[(0,E.jsxs)(`div`,{className:`stabilizer-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN25 / one branch, four routes`}),(0,E.jsx)(`h2`,{children:`Normalization, dropout, and residual paths`}),(0,E.jsx)(`p`,{children:`Hold one learned branch fixed, then watch each training mechanism change its forward values and reverse gradient.`})]}),(0,E.jsx)(`div`,{className:`stabilizer-chip`,children:`4 coordinates`})]}),(0,E.jsxs)(`section`,{className:`stabilizer-common-panel`,"aria-label":`Shared stabilizer branch`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Shared setup`}),(0,E.jsx)(`h2`,{children:`Everything starts from the same branch`})]}),(0,E.jsx)(`span`,{children:`score = upstream · output`})]}),(0,E.jsxs)(`div`,{className:`stabilizer-common-flow`,children:[(0,E.jsx)(Kr,{label:`input x`,values:i.input,selectedCoordinate:s}),(0,E.jsxs)(`div`,{className:`stabilizer-flow-arrow`,children:[`× `,j(i.branchWeight)]}),(0,E.jsx)(Kr,{label:`learned branch h`,values:i.branch,selectedCoordinate:s,tone:`purple`}),(0,E.jsx)(Kr,{label:`upstream dS/doutput`,values:i.upstreamGradient,selectedCoordinate:s,tone:`red`})]})]}),(0,E.jsxs)(`section`,{className:`stabilizer-comparison-panel`,"aria-label":`Training stabilizer route comparison`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same numbers, different jobs`}),(0,E.jsx)(`h2`,{children:`Compare all four routes`})]}),(0,E.jsx)(`span`,{children:`select a route to unpack it`})]}),(0,E.jsx)(`div`,{className:`stabilizer-comparison-grid`,children:i.routes.map(n=>{let r=Fr.find(e=>e.id===n.id);return(0,E.jsxs)(`button`,{"aria-pressed":n.id===e,type:`button`,onClick:()=>t(n.id),children:[(0,E.jsx)(`strong`,{children:r.label}),(0,E.jsx)(`span`,{children:r.summary}),(0,E.jsxs)(`code`,{children:[`output `,Gr(n.output)]}),(0,E.jsxs)(`code`,{children:[`dS/dx `,Gr(n.inputGradient)]}),(0,E.jsxs)(`small`,{children:[`score `,j(n.score)]})]},n.id)})})]}),(0,E.jsxs)(`section`,{className:`stabilizer-forward-panel`,"aria-label":`Selected stabilizer forward trace`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Forward / `,o.label]}),(0,E.jsx)(`h2`,{children:`What changes on this route?`})]}),(0,E.jsxs)(`strong`,{children:[`score `,j(a.score)]})]}),e===`normalization`?(0,E.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,E.jsxs)(`div`,{className:`stabilizer-stat-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean`}),(0,E.jsx)(`strong`,{children:j(i.normalization.mean)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`variance / population`}),(0,E.jsx)(`strong`,{children:j(i.normalization.variance)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`standard deviation`}),(0,E.jsx)(`strong`,{children:j(i.normalization.standardDeviation)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`epsilon / hand fixture`}),(0,E.jsx)(`strong`,{children:`0`})]})]}),(0,E.jsx)(Kr,{label:`centered h - mean`,values:i.normalization.centered,selectedCoordinate:s,tone:`purple`}),(0,E.jsx)(Kr,{label:`normalized output`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,E.jsx)(`code`,{className:`stabilizer-formula`,children:`normalized[i] = (h[i] - mean) / standard deviation`})]}):e===`dropout`?(0,E.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,E.jsx)(Kr,{label:`binary mask`,values:i.dropoutMask,selectedCoordinate:s,tone:`red`}),(0,E.jsx)(Kr,{label:`mask / keep probability`,values:i.dropout.scaledMask,selectedCoordinate:s,tone:`purple`}),(0,E.jsx)(Kr,{label:`training output`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,E.jsxs)(`div`,{className:`stabilizer-dropout-compare`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`evaluation / dropout off`}),(0,E.jsx)(`code`,{children:Gr(i.dropout.evaluationOutput)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`expectation over training masks`}),(0,E.jsx)(`code`,{children:Gr(i.dropout.trainingExpectation)})]})]}),(0,E.jsxs)(`code`,{className:`stabilizer-formula`,children:[`training output[i] = h[i] × mask[i] / `,j(i.keepProbability)]})]}):e===`residual`?(0,E.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,E.jsx)(Kr,{label:`identity skip x`,values:i.input,selectedCoordinate:s}),(0,E.jsx)(`div`,{className:`stabilizer-plus`,children:`+`}),(0,E.jsx)(Kr,{label:`learned branch h`,values:i.branch,selectedCoordinate:s,tone:`purple`}),(0,E.jsx)(`div`,{className:`stabilizer-plus`,children:`=`}),(0,E.jsx)(Kr,{label:`residual output`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,E.jsx)(`code`,{className:`stabilizer-formula`,children:`output[i] = input[i] + branch[i]`})]}):(0,E.jsxs)(`div`,{className:`stabilizer-mechanism-trace`,children:[(0,E.jsx)(Kr,{label:`plain output = h`,values:a.output,selectedCoordinate:s,tone:`green`}),(0,E.jsxs)(`code`,{className:`stabilizer-formula`,children:[`No extra route: output[i] = `,j(i.branchWeight),` × input[i]`]})]})]}),(0,E.jsxs)(`section`,{className:`stabilizer-backward-panel`,"aria-label":`Selected stabilizer backward trace`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Backward / vector-Jacobian product`}),(0,E.jsx)(`h2`,{children:`Where does the score gradient travel?`})]}),(0,E.jsxs)(`span`,{children:[`dS/dweight `,j(a.weightGradient)]})]}),(0,E.jsxs)(`div`,{className:`stabilizer-gradient-flow`,children:[(0,E.jsx)(Kr,{label:`upstream`,values:i.upstreamGradient,selectedCoordinate:s,tone:`red`}),(0,E.jsx)(Kr,{label:`into learned branch`,values:a.branchGradient,selectedCoordinate:s,tone:`purple`}),e===`residual`?(0,E.jsx)(Kr,{label:`through identity skip`,values:a.skipGradient,selectedCoordinate:s}):null,(0,E.jsx)(Kr,{label:`total dS/dinput`,values:a.inputGradient,selectedCoordinate:s,tone:`green`})]})]}),(0,E.jsxs)(`section`,{className:`stabilizer-arithmetic-panel`,"aria-label":`Selected stabilizer coordinate calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Open coordinate `,s+1]}),(0,E.jsx)(`h2`,{children:`One reverse calculation`})]}),(0,E.jsxs)(`span`,{children:[`input `,j(i.input[s])]})]}),(0,E.jsxs)(`div`,{className:`stabilizer-equations`,children:[e===`normalization`?(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`code`,{children:[`(4 × `,j(i.upstreamGradient[s]),` - `,j(i.normalization.upstreamSum),` - `,j(i.normalization.normalized[s]),` × `,j(i.normalization.upstreamDotNormalized),`) / (4 × `,j(i.normalization.standardDeviation),`) = `,j(a.branchGradient[s])]}),(0,E.jsx)(`span`,{children:`layer norm couples this coordinate to both vector-wide sums`})]}):e===`dropout`?(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`code`,{children:[j(i.upstreamGradient[s]),` × `,j(i.dropoutMask[s]),` / `,j(i.keepProbability),` = `,j(a.branchGradient[s])]}),(0,E.jsx)(`span`,{children:`a dropped coordinate receives zero branch gradient`})]}):(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`code`,{children:[`dS/dh[`,s+1,`] = `,j(a.branchGradient[s])]}),(0,E.jsx)(`span`,{children:e===`residual`?`the branch and skip both receive the upstream gradient`:`the plain branch passes the upstream gradient unchanged`})]}),(0,E.jsxs)(`code`,{children:[j(i.branchWeight),` × `,j(a.branchGradient[s]),` + `,j(a.skipGradient[s]),` = `,j(a.inputGradient[s])]}),(0,E.jsx)(`span`,{children:`branch contribution + identity-skip contribution`}),(0,E.jsxs)(`code`,{children:[`Σ dS/dh[i] × input[i] = `,j(a.weightGradient)]}),(0,E.jsx)(`span`,{children:`the shared scalar branch-weight gradient`})]})]}),(0,E.jsxs)(`section`,{className:`stabilizer-audit-panel`,"aria-label":`Training stabilizer finite difference audit`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Independent numerical audit`}),(0,E.jsx)(`h2`,{children:`Analytical gradients match score slopes`})]}),(0,E.jsx)(`span`,{children:`epsilon 1e-6`})]}),(0,E.jsxs)(`div`,{className:`stabilizer-audit-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`selected analytical dS/dx`}),(0,E.jsx)(`code`,{children:j(a.inputGradient[s])})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`selected finite difference`}),(0,E.jsx)(`code`,{children:j(a.finiteDifferenceInputGradient[s])})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`maximum input error`}),(0,E.jsx)(`code`,{children:j(c)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`analytical dS/dweight`}),(0,E.jsx)(`code`,{children:j(a.weightGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`weight finite difference`}),(0,E.jsx)(`code`,{children:j(a.finiteDifferenceWeightGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`weight error`}),(0,E.jsx)(`code`,{children:j(a.weightGradientAbsoluteError)})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`controls stabilizer-controls`,"aria-label":`Training stabilizer controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Training mechanism`}),(0,E.jsx)(`h2`,{children:`Choose a route`}),(0,E.jsx)(`p`,{children:`The learned branch, input, and upstream vector stay fixed.`}),(0,E.jsx)(`div`,{className:`stabilizer-route-buttons`,children:Fr.map(n=>(0,E.jsxs)(`button`,{"aria-pressed":n.id===e,type:`button`,onClick:()=>t(n.id),children:[(0,E.jsx)(`strong`,{children:n.label}),(0,E.jsx)(`span`,{children:n.summary})]},n.id))}),(0,E.jsx)(`p`,{className:`eyebrow`,children:`Coordinate microscope`}),(0,E.jsx)(`div`,{className:`stabilizer-coordinate-buttons`,children:i.input.map((e,t)=>(0,E.jsxs)(`button`,{"aria-label":`Open stabilizer coordinate ${t+1}`,"aria-pressed":s===t,type:`button`,onClick:()=>r(t),children:[(0,E.jsx)(`span`,{children:t+1}),(0,E.jsxs)(`code`,{children:[`x = `,j(e)]})]},t))}),(0,E.jsxs)(`div`,{className:`stabilizer-reading`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Different jobs`}),(0,E.jsx)(`h2`,{children:e===`normalization`?`Coordinates share context`:e===`dropout`?`Training samples a subnetwork`:e===`residual`?`The skip keeps a short route`:`The control exposes the branch`}),(0,E.jsx)(`p`,{children:`These mechanisms can coexist, but they are not interchangeable fixes for depth.`})]})]})]})}function Jr(){let[e,t]=(0,l.useState)(`initialization`);return(0,E.jsxs)(`div`,{className:`deep-training-workbench`,children:[(0,E.jsxs)(`nav`,{className:`deep-training-switch`,"aria-label":`Deep training learning lab`,children:[(0,E.jsx)(`button`,{"aria-pressed":e===`initialization`,type:`button`,onClick:()=>t(`initialization`),children:`Initialization`}),(0,E.jsx)(`button`,{"aria-pressed":e===`gradient-flow`,type:`button`,onClick:()=>t(`gradient-flow`),children:`Gradient flow`}),(0,E.jsx)(`button`,{"aria-pressed":e===`stabilizers`,type:`button`,onClick:()=>t(`stabilizers`),children:`Stabilizers`})]}),e===`initialization`?(0,E.jsx)(Pr,{}):e===`gradient-flow`?(0,E.jsx)(Cr,{}):(0,E.jsx)(qr,{})]})}var Yr=4,Xr=12,Zr=1e6,Qr=/^[a-z][a-z0-9_]{0,31}$/,$r=[{id:`multiply_add_square`,title:`Complete graph`,summary:`Multiply, add, and square with every saved value visible.`,expression:`loss = (x × w + b)²`,inputs:[{id:`x`,value:2,requiresGradient:!0},{id:`w`,value:3,requiresGradient:!0},{id:`b`,value:1,requiresGradient:!0}],steps:[{id:`m`,operation:`multiply`,inputs:[`x`,`w`]},{id:`z`,operation:`add`,inputs:[`m`,`b`]},{id:`loss`,operation:`square`,inputs:[`z`]}],output:`loss`,mutationsAfterForward:{}},{id:`negative_branch`,title:`Runtime branch`,summary:`A negative input records negate, not the unexecuted identity path.`,expression:`loss = abs(x)², x < 0`,inputs:[{id:`x`,value:-2,requiresGradient:!0}],steps:[{id:`abs_x`,operation:`branch_nonnegative`,inputs:[`x`]},{id:`loss`,operation:`square`,inputs:[`abs_x`]}],output:`loss`,mutationsAfterForward:{}},{id:`saved_snapshot`,title:`Mutation snapshot`,summary:`Live w becomes 100; backward still reads saved forward w = 3.`,expression:`product = x × w; then live w ← 100`,inputs:[{id:`x`,value:2,requiresGradient:!0},{id:`w`,value:3,requiresGradient:!0}],steps:[{id:`product`,operation:`multiply`,inputs:[`x`,`w`]}],output:`product`,mutationsAfterForward:{w:100}}];function ei(e,t){if(!Number.isFinite(e))throw Error(`${t} must remain finite`);return e}function ti(e,t){if(typeof e!=`string`||!Qr.test(e))throw Error(`${t} must be a bounded identifier`)}function ni(e,t,n=0){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>Zr+n)throw Error(`${t} must be finite and bounded`)}function ri(e){let t=Object.create(null);Object.entries(e.mutationsAfterForward).forEach(([e,n])=>{t[e]=n});let n=e.inputs.map(e=>Object.freeze({...e})),r=e.steps.map(e=>Object.freeze({...e,inputs:Object.freeze([...e.inputs])}));return Object.freeze({id:e.id,title:e.title,summary:e.summary,expression:e.expression,inputs:Object.freeze(n),steps:Object.freeze(r),output:e.output,mutationsAfterForward:Object.freeze(t)})}function ii(e){return e===`multiply`||e===`add`?2:1}function ai(e){if(typeof e!=`object`||!e||!Array.isArray(e.inputs)||!Array.isArray(e.steps)||typeof e.mutationsAfterForward!=`object`||e.mutationsAfterForward===null||Array.isArray(e.mutationsAfterForward))throw Error(`autograd scenario must contain bounded arrays and mutation object`);if(e.inputs.length<1||e.inputs.length>Yr||e.steps.length<1||e.steps.length>Xr)throw Error(`autograd scenario exceeds the bounded graph size`);let t=new Set;if(e.inputs.forEach((e,n)=>{if(typeof e!=`object`||!e)throw Error(`input must be an object`);if(ti(e.id,`input ${n} id`),ni(e.value,`input ${e.id}`),e.requiresGradient!==!0||t.has(e.id))throw Error(`inputs must require gradients and have unique ids`);t.add(e.id)}),e.steps.forEach((e,n)=>{if(typeof e!=`object`||!e||!Array.isArray(e.inputs))throw Error(`step must contain an inputs array`);if(ti(e.id,`step ${n} id`),t.has(e.id)||![`multiply`,`add`,`square`,`negate`,`branch_nonnegative`].includes(e.operation))throw Error(`step id or operation is invalid`);if(e.inputs.length!==ii(e.operation))throw Error(`${e.operation} has invalid arity`);e.inputs.forEach(n=>{if(ti(n,`step ${e.id} parent`),!t.has(n))throw Error(`step ${e.id} parent must already exist`)}),t.add(e.id)}),e.output!==e.steps.at(-1).id)throw Error(`autograd output must be the final executed step`);let n=new Set(e.inputs.map(e=>e.id)),r=Object.entries(e.mutationsAfterForward);if(r.length>Yr)throw Error(`too many live mutations`);r.forEach(([e,t])=>{if(ti(e,`mutation id`),ni(t,`mutation ${e}`),!n.has(e))throw Error(`mutation ${e} must target an input`)})}function oi(e,t={},n=0){let r=[],i=new Map,a=Object.create(null);return e.inputs.forEach(e=>{let a=Object.prototype.hasOwnProperty.call(t,e.id)?t[e.id]:e.value;ni(a,`input ${e.id}`,n);let o={id:e.id,operation:`input`,parents:[],forwardValue:a,savedValues:[]};r.push(o),i.set(o.id,o)}),e.steps.forEach(e=>{let t=e.inputs.map(e=>i.get(e)),n=t.map(e=>e.forwardValue),o=e.operation,s,c=[];e.operation===`multiply`?(s=ei(n[0]*n[1],`${e.id} product`),c=[{name:`left`,sourceId:t[0].id,value:n[0]},{name:`right`,sourceId:t[1].id,value:n[1]}]):e.operation===`add`?s=ei(n[0]+n[1],`${e.id} sum`):e.operation===`square`?(s=ei(n[0]*n[0],`${e.id} square`),c=[{name:`input`,sourceId:t[0].id,value:n[0]}]):e.operation===`negate`?s=ei(-n[0],`${e.id} negation`):n[0]>=0?(o=`identity`,a[e.id]=`nonnegative`,s=n[0]):(o=`negate`,a[e.id]=`negative`,s=ei(-n[0],`${e.id} branch negation`));let l={id:e.id,operation:o,parents:[...e.inputs],forwardValue:s,savedValues:c};r.push(l),i.set(l.id,l)}),{nodes:r,branches:a}}function si(e,t){let n=new Map(e.map(e=>[e.id,e])),r=new Set,i=[];function a(e){r.has(e)||(r.add(e),n.get(e).parents.forEach(a),i.push(e))}return a(t),i}function ci(e,t){let n=e.savedValues.find(e=>e.name===t);if(!n)throw Error(`${e.id} is missing saved ${t}`);return n.value}function li(e){if(e.operation===`multiply`)return[{parentId:e.parents[0],value:ci(e,`right`),source:`saved:right`},{parentId:e.parents[1],value:ci(e,`left`),source:`saved:left`}];if(e.operation===`add`)return[{parentId:e.parents[0],value:1,source:`constant:1`},{parentId:e.parents[1],value:1,source:`constant:1`}];if(e.operation===`square`)return[{parentId:e.parents[0],value:ei(2*ci(e,`input`),`${e.id} derivative`),source:`saved:input`}];if(e.operation===`negate`)return[{parentId:e.parents[0],value:-1,source:`constant:-1`}];if(e.operation===`identity`)return[{parentId:e.parents[0],value:1,source:`constant:1`}];throw Error(`cannot differentiate ${e.operation}`)}function ui(e,t,n){return oi(e,t,n).nodes.at(-1).forwardValue}function di(e,t=1e-5,n=!0){if(ai(e),!Number.isFinite(t)||t<1e-12||t>1)throw Error(`finite-difference epsilon must be finite and in [1e-12, 1]`);let r=ri(e),{nodes:i,branches:a}=oi(r),o=new Map(i.map(e=>[e.id,e])),s=si(i,r.output),c=[...s].reverse(),l=Object.create(null);l[r.output]=1;let u=[];c.forEach(e=>{let t=o.get(e),n=l[e];if(n===void 0||t.operation===`input`)return;let r=li(t),i=r.map(e=>{let r=ei(n*e.value,`${t.id} parent contribution`);return l[e.parentId]=ei((l[e.parentId]??0)+r,`${e.parentId} accumulated gradient`),{parentId:e.parentId,value:r}});u.push({nodeId:e,operation:t.operation,upstreamGradient:n,localDerivatives:r,parentContributions:i})});let d=Object.fromEntries(r.inputs.map(e=>[e.id,e.value])),f=Object.create(null),p=Object.create(null);return r.inputs.forEach(e=>{let n={...d,[e.id]:e.value+t},i={...d,[e.id]:e.value-t},a=ei((ui(r,n,t)-ui(r,i,t))/(2*t),`${e.id} finite difference`);f[e.id]=a,p[e.id]=Math.abs(l[e.id]-a)}),{scenario:r,nodes:i,topologicalOrder:s,backwardOrder:c,branchChoices:a,liveInputValues:n?{...d,...r.mutationsAfterForward}:d,backwardSteps:u,gradients:l,finiteDifferenceGradients:f,gradientAbsoluteErrors:p,maxGradientAbsoluteError:Math.max(...Object.values(p),0)}}function fi(e,t=!0){let n=$r.find(t=>t.id===e);if(!n)throw Error(`unknown dynamic autograd scenario: ${e}`);return di(n,1e-5,t)}function pi(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function mi(e){return e===`input`?`leaf input`:e}function hi(e){return e.operation===`input`?`${e.id} entered the graph as a leaf`:e.operation===`multiply`?`${e.id} = ${e.parents[0]} × ${e.parents[1]}`:e.operation===`add`?`${e.id} = ${e.parents[0]} + ${e.parents[1]}`:e.operation===`square`?`${e.id} = ${e.parents[0]}²`:e.operation===`negate`?`${e.id} = -${e.parents[0]}`:`${e.id} = identity(${e.parents[0]})`}function gi(){let[e,t]=(0,l.useState)(`multiply_add_square`),[n,r]=(0,l.useState)(`m`),[i,a]=(0,l.useState)(0),[o,s]=(0,l.useState)(!0),c=(0,l.useMemo)(()=>fi(e,o),[e,o]),u=c.nodes.find(e=>e.id===n)??c.nodes.at(-1),d=c.backwardSteps[Math.min(i,c.backwardSteps.length-1)],f=Object.keys(c.scenario.mutationsAfterForward).length>0;function p(e){let n=$r.find(t=>t.id===e);t(e),r(n.steps[0].id),a(0),s(!0)}return(0,E.jsxs)(`main`,{className:`workspace workspace--dynamic-autograd`,children:[(0,E.jsxs)(`section`,{className:`autograd-stage`,"aria-label":`Dynamic autograd and saved value visualizer`,children:[(0,E.jsxs)(`section`,{className:`autograd-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN27 / tensor and autograd bridge`}),(0,E.jsx)(`h2`,{children:`Dynamic graph and saved-value microscope`}),(0,E.jsx)(`p`,{children:`The forward run records only executed operations. Backward reverses that graph and reads immutable forward snapshots.`})]}),(0,E.jsx)(`div`,{className:`autograd-chip`,children:`reverse mode`})]}),(0,E.jsxs)(`section`,{className:`autograd-graph-panel`,"aria-label":`Executed dynamic computation graph`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 1 / record what ran`}),(0,E.jsx)(`h2`,{children:c.scenario.expression})]}),(0,E.jsxs)(`span`,{children:[c.nodes.length,` executed nodes`]})]}),(0,E.jsxs)(`div`,{className:`autograd-order-strip`,children:[(0,E.jsx)(`small`,{children:`topological order`}),(0,E.jsx)(`code`,{children:c.topologicalOrder.join(` → `)})]}),(0,E.jsx)(`div`,{className:`autograd-node-lane`,children:c.nodes.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Open node ${e.id}, ${mi(e.operation)}, value ${pi(e.forwardValue)}`,"aria-pressed":e.id===u.id,type:`button`,onClick:()=>r(e.id),children:[(0,E.jsx)(`small`,{children:mi(e.operation)}),(0,E.jsxs)(`strong`,{children:[e.id,` = `,pi(e.forwardValue)]}),(0,E.jsx)(`span`,{children:e.parents.length?`from ${e.parents.join(` + `)}`:`leaf`})]},e.id))}),Object.entries(c.branchChoices).map(([e,t])=>(0,E.jsxs)(`div`,{className:`autograd-branch-note`,children:[(0,E.jsx)(`strong`,{children:e}),` chose the `,(0,E.jsx)(`code`,{children:t}),` branch. The other operation is absent from this graph.`]},e))]}),(0,E.jsxs)(`section`,{className:`autograd-saved-panel`,"aria-label":`Selected node forward and saved value trace`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 2 / save the derivative ingredients`}),(0,E.jsxs)(`h2`,{children:[`Open node `,u.id]})]}),(0,E.jsx)(`span`,{children:mi(u.operation)})]}),(0,E.jsxs)(`div`,{className:`autograd-selected-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`forward rule`}),(0,E.jsx)(`code`,{children:hi(u)}),(0,E.jsxs)(`strong`,{children:[`value `,pi(u.forwardValue)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`saved for backward`}),u.savedValues.length?u.savedValues.map(e=>(0,E.jsxs)(`code`,{children:[e.name,` ← `,e.sourceId,` = `,pi(e.value)]},e.name)):(0,E.jsx)(`code`,{children:`nothing — local derivative is constant`})]})]}),f?(0,E.jsxs)(`div`,{className:`autograd-mutation-strip`,children:[c.scenario.inputs.map(e=>{let t=c.liveInputValues[e.id];return(0,E.jsxs)(`div`,{className:t===e.value?``:`is-mutated`,children:[(0,E.jsx)(`small`,{children:e.id}),(0,E.jsxs)(`code`,{children:[`forward `,pi(e.value)]}),(0,E.jsxs)(`strong`,{children:[`live `,pi(t)]})]},e.id)}),(0,E.jsx)(`p`,{children:`Backward reads the saved forward snapshots, never the later live value.`})]}):null]}),(0,E.jsxs)(`section`,{className:`autograd-backward-panel`,"aria-label":`Reverse topological backward trace`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 3 / reverse the executed graph`}),(0,E.jsx)(`h2`,{children:`Upstream × local derivative`})]}),(0,E.jsx)(`span`,{children:c.backwardOrder.join(` ← `)})]}),(0,E.jsx)(`div`,{className:`autograd-backward-buttons`,children:c.backwardSteps.map((e,t)=>(0,E.jsxs)(`button`,{"aria-label":`Open backward node ${e.nodeId}, upstream ${pi(e.upstreamGradient)}`,"aria-pressed":t===i,type:`button`,onClick:()=>a(t),children:[(0,E.jsx)(`small`,{children:e.operation}),(0,E.jsx)(`strong`,{children:e.nodeId}),(0,E.jsxs)(`code`,{children:[`upstream `,pi(e.upstreamGradient)]})]},e.nodeId))}),(0,E.jsx)(`div`,{className:`autograd-backward-equations`,"aria-label":`Selected backward calculation`,children:d.localDerivatives.map((e,t)=>{let n=d.parentContributions[t];return(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`toward `,e.parentId]}),(0,E.jsxs)(`code`,{children:[pi(d.upstreamGradient),` × `,pi(e.value),` = `,pi(n.value)]}),(0,E.jsxs)(`span`,{children:[`local source: `,e.source]})]},`${d.nodeId}-${e.parentId}`)})})]}),(0,E.jsxs)(`section`,{className:`autograd-audit-panel`,"aria-label":`Dynamic autograd finite difference audit`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 4 / distrust the graph once`}),(0,E.jsx)(`h2`,{children:`Fresh forwards check every leaf`})]}),(0,E.jsx)(`span`,{children:`epsilon 1e-5`})]}),(0,E.jsxs)(`div`,{className:`autograd-audit-grid`,children:[c.scenario.inputs.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`strong`,{children:e.id}),(0,E.jsxs)(`span`,{children:[`analytical `,(0,E.jsx)(`code`,{children:pi(c.gradients[e.id])})]}),(0,E.jsxs)(`span`,{children:[`numerical `,(0,E.jsx)(`code`,{children:pi(c.finiteDifferenceGradients[e.id])})]}),(0,E.jsxs)(`small`,{children:[`error `,pi(c.gradientAbsoluteErrors[e.id])]})]},e.id)),(0,E.jsxs)(`div`,{className:`autograd-audit-max`,children:[(0,E.jsx)(`strong`,{children:`maximum error`}),(0,E.jsx)(`code`,{children:pi(c.maxGradientAbsoluteError)}),(0,E.jsx)(`small`,{children:`must stay below 1e-8`})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`controls autograd-controls`,"aria-label":`Dynamic autograd scenarios`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Graph presets`}),(0,E.jsx)(`h2`,{children:`Change one graph rule`}),(0,E.jsx)(`div`,{className:`autograd-scenario-buttons`,children:$r.map(t=>(0,E.jsxs)(`button`,{"aria-pressed":t.id===e,type:`button`,onClick:()=>p(t.id),children:[(0,E.jsx)(`strong`,{children:t.title}),(0,E.jsx)(`code`,{children:t.expression}),(0,E.jsx)(`span`,{children:t.summary})]},t.id))}),f?(0,E.jsx)(`button`,{className:`autograd-mutation-toggle`,"aria-pressed":o,type:`button`,onClick:()=>s(e=>!e),children:o?`Restore forward-time live values`:`Apply post-forward mutation`}):null,(0,E.jsxs)(`div`,{className:`autograd-mental-model`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Keep this picture`}),(0,E.jsx)(`h2`,{children:`Record, save, reverse.`}),(0,E.jsx)(`p`,{children:`Record only executed operations. Save only derivative ingredients. Reverse children before parents.`})]})]})]})}var _i=/^[A-Za-z][A-Za-z0-9_]{0,63}$/,vi=8,yi=1e3,bi=0xe8d4a51000,xi=[{id:`single_row`,title:`One row by hand`,summary:`Follow 4 and 8 through every graph, NeuralIR, and MatrixIR value.`,inputs:{x0:[4],x1:[8]}},{id:`two_row_batch`,title:`The same plan, two rows`,summary:`Keep the lowered program fixed while its input columns grow to two rows.`,inputs:{x0:[4,8],x1:[8,16]}}],Si=[{id:`x0`,op:`input`,detail:`runtime input x0`},{id:`x1`,op:`input`,detail:`runtime input x1`},{id:`bias`,op:`constant`,detail:`constant 1`},{id:`sum`,op:`weighted_sum`,detail:`three weighted terms`},{id:`relu`,op:`activation`,detail:`max(0, sum)`},{id:`out`,op:`output`,detail:`prediction`}],Ci=[{id:`w0`,from:`x0`,to:`sum`,weight:.25},{id:`w1`,from:`x1`,to:`sum`,weight:.75},{id:`bias_to_sum`,from:`bias`,to:`sum`,weight:-1},{id:`sum_to_relu`,from:`sum`,to:`relu`,weight:1},{id:`relu_to_out`,from:`relu`,to:`out`,weight:1}];function wi(e,t,n=512){if(typeof e!=`string`||e.length<1||e.length>n)throw Error(`${t} must be a bounded string`);return e}function Ti(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>yi)throw Error(`${t} must be finite and bounded`);return e}function Ei(e,t){if(!Number.isFinite(e)||Math.abs(e)>bi)throw Error(`${t} must remain finite and bounded`);return e}function Di(e){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`forward lowering scenario must be an object`);if(Object.keys(e).sort().join(`,`)!==`id,inputs,summary,title`)throw Error(`forward lowering scenario has an unexpected field`);let t=wi(e.id,`scenario id`,64);if(!_i.test(t))throw Error(`scenario id must be a bounded identifier`);let n=wi(e.title,`scenario title`),r=wi(e.summary,`scenario summary`);if(typeof e.inputs!=`object`||e.inputs===null||Array.isArray(e.inputs))throw Error(`scenario inputs must be an object`);if(Object.keys(e.inputs).sort().join(`,`)!==`x0,x1`)throw Error(`scenario inputs must contain exactly x0 and x1`);let i=e.inputs.x0,a=e.inputs.x1;if(!Array.isArray(i)||!Array.isArray(a))throw Error(`scenario input columns must be arrays`);if(i.length<1||i.length>vi||a.length!==i.length)throw Error(`scenario input columns must have the same bounded length`);return Ri({id:t,title:n,summary:r,inputs:{x0:i.map((e,t)=>Ti(e,`x0[${t}]`)),x1:a.map((e,t)=>Ti(e,`x1[${t}]`))}})}function Oi(){let e=Qe(`tiny-weighted-relu`);return $e(e,`x0`),$e(e,`x1`),et(e,`bias`,1),tt(e,`sum`,[{from:`x0`,weight:.25,edgeId:`w0`},{from:`x1`,weight:.75,edgeId:`w1`},{from:`bias`,weight:-1,edgeId:`bias_to_sum`}]),nt(e,`relu`,`sum`,`relu`,{},`sum_to_relu`),rt(e,`out`,`relu`,`prediction`,{},`relu_to_out`),e}function ki(e){return e.op===`MUL`?[e.left,e.right]:e.op===`ADD`?[...e.inputs??[]]:e.op===`ACTIVATE`||e.op===`STORE_OUTPUT`?[e.input]:[]}function Ai(e){switch(e.op){case`LOAD_INPUT`:return{input_name:e.inputName};case`LOAD_CONST`:return{value:e.value??0};case`LOAD_EDGE_WEIGHT`:return{edge_id:e.edgeId};case`ACTIVATE`:return{activation:e.activation??`relu`};case`STORE_OUTPUT`:return{output_name:e.outputName??`output`};default:return{}}}function ji(e){return e.map((e,t)=>({id:`i${t}`,op:e.op,output:e.dst??null,inputs:ki(e),attributes:Ai(e),sourceNodes:e.sourceNode===void 0?[]:[e.sourceNode],sourceEdges:e.sourceEdge===void 0?[]:[e.sourceEdge]}))}function Mi(e,t){let n=new Set((e.terms??[]).map(e=>e.edgeId));return t.filter(t=>t.sourceEdges.some(e=>n.has(e))||t.op===`ADD`&&t.output===e.dst).map(e=>e.id)}function Ni(e,t){return e.map((e,n)=>{let r=e.op===`WEIGHTED_SUM_MATRIX`,i=e.terms??[],a=r?i.map(e=>e.sourceValue):e.input===void 0?[]:[e.input],o={};return e.op===`LOAD_INPUT_MATRIX`&&(o.input_name=e.inputName),e.op===`LOAD_CONST_MATRIX`&&(o.value=e.value??0),r&&(o.edge_ids=i.map(e=>e.edgeId),o.weights=i.map(e=>e.weight)),e.op===`ACTIVATE_MATRIX`&&(o.activation=e.activation??`relu`),e.op===`STORE_OUTPUT_MATRIX`&&(o.output_name=e.outputName??`output`),{id:`m${n}`,op:e.op,output:e.dst??null,inputs:a,attributes:o,sourceInstructions:r?Mi(e,t):e.sourceInstructionIndexes.map(e=>`i${e}`),sourceNodes:e.sourceNode===void 0?[]:[e.sourceNode],sourceEdges:r?i.map(e=>e.edgeId):[]}})}function Pi(e){return e.inputs.x0.map((t,n)=>{let r=Ei(Ei(-1,`bias term`)+Ei(t*.25,`x0 term`)+Ei(e.inputs.x1[n]*.75,`x1 term`),`direct row ${n}`);return Math.max(0,r)})}function Fi(e){let t=0;for(let n=0;n<e[0].length;n+=1)for(let r=0;r<e.length;r+=1)for(let i=r+1;i<e.length;i+=1)t=Math.max(t,Math.abs(e[r][n]-e[i][n]));return Ei(t,`parity error`)}function Ii(e){let t=Di(e),n=ot(Oi()),r=n.functions[0],i=Ct(n),a=ji(r.instructions),o=Ni(i.instructions,a),s=t.inputs.x0.map((e,r)=>ct(n,{x0:e,x1:t.inputs.x1[r]})),c=s.map(e=>Ei(e.outputs.prediction,`NeuralIR output`)),l=s.map(e=>Object.values(e.values).map(e=>Ei(e,`NeuralIR value`))),u=Tt(i,t.inputs),d=(u.outputs.prediction??[]).map(e=>Ei(e,`MatrixIR output`)),f=Object.entries(u.values).map(([e,t])=>({valueId:e,values:t.map(t=>Ei(t,`MatrixIR ${e}`))})),p=Pi(t),m=Fi([p,c,d]),h=s[0].instructions.map((e,t)=>({instructionId:`i${t}`,reads:e.reads.map(e=>({...e})),write:e.write===void 0?void 0:{...e.write},output:e.output===void 0?void 0:{...e.output}}));return Ri({scenario:t,graph:{nodes:Si.map(e=>({...e})),edges:Ci.map(e=>({...e})),topologicalOrder:[`bias`,`x0`,`x1`,`sum`,`relu`,`out`]},neuralIr:{magic:`CANN`,version:0,instructions:a},matrixIr:{magic:`CANM`,version:0,sourceNeuralIrVersion:0,operations:o},directOutputs:p,neuralIrOutputs:c,matrixIrOutputs:d,neuralValueRows:l,matrixValueColumns:f,firstRowInstructionReadings:h,maxParityError:m})}function Li(e){let t=xi.find(t=>t.id===e);if(t===void 0)throw Error(`unknown forward lowering scenario: ${e}`);return Ii(t)}function Ri(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>Ri(e)),e)}function zi(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(10)).toString()}function Bi(e){switch(e.op){case`LOAD_CONST`:return`materialize ${e.attributes.value}`;case`LOAD_INPUT`:return`bind ${e.attributes.input_name}`;case`LOAD_EDGE_WEIGHT`:return`load ${e.attributes.edge_id}`;case`MUL`:return`${e.inputs.join(` x `)}`;case`ADD`:return e.inputs.join(` + `);case`ACTIVATE`:return`${e.attributes.activation}(${e.inputs[0]})`;case`STORE_OUTPUT`:return`publish ${e.attributes.output_name}`;default:return e.op}}function Vi(e){switch(e.op){case`LOAD_CONST_MATRIX`:return`broadcast ${e.attributes.value}`;case`LOAD_INPUT_MATRIX`:return`column ${e.attributes.input_name}`;case`WEIGHTED_SUM_MATRIX`:return`${e.inputs.length} fused terms`;case`ACTIVATE_MATRIX`:return`${e.attributes.activation} column`;case`STORE_OUTPUT_MATRIX`:return`publish ${e.attributes.output_name}`;default:return e.op}}function M(e){let t=Object.entries(e);return t.length===0?`none`:t.map(([e,t])=>`${e}=${Array.isArray(t)?`[${t.join(`, `)}]`:String(t)}`).join(`; `)}function Hi(){let[e,t]=(0,l.useState)(`single_row`),[n,r]=(0,l.useState)({lane:`matrix`,id:`m3`}),i=(0,l.useMemo)(()=>Li(e),[e]),a=n.lane===`neural`?i.neuralIr.instructions.find(e=>e.id===n.id):void 0,o=n.lane===`matrix`?i.matrixIr.operations.find(e=>e.id===n.id):void 0,s=a===void 0?void 0:i.firstRowInstructionReadings.find(e=>e.instructionId===a.id);return(0,E.jsxs)(`main`,{className:`workspace workspace--forward-lowering`,children:[(0,E.jsxs)(`section`,{className:`forward-lowering-stage`,children:[(0,E.jsxs)(`header`,{className:`forward-lowering-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN29 - graph -> NeuralIR -> MatrixIR`}),(0,E.jsx)(`h2`,{children:`Forward graph lowering map`}),(0,E.jsx)(`p`,{children:`Keep one prediction fixed while a dependency graph becomes an ordered scalar program and then a fused batch plan.`})]}),(0,E.jsxs)(`span`,{className:`forward-lowering-chip`,children:[`6 nodes -> `,i.neuralIr.instructions.length,` instructions -> `,i.matrixIr.operations.length,` ops`]})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-graph`,"aria-label":`Canonical forward neural graph`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`1 - meaning`}),(0,E.jsx)(`h2`,{children:`The graph says what depends on what`})]}),(0,E.jsx)(`code`,{children:i.graph.topologicalOrder.join(` -> `)})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-node-flow`,children:[(0,E.jsx)(`div`,{className:`forward-lowering-input-stack`,children:i.graph.nodes.slice(0,3).map(e=>(0,E.jsxs)(`article`,{children:[(0,E.jsx)(`strong`,{children:e.id}),(0,E.jsx)(`span`,{children:e.detail})]},e.id))}),(0,E.jsx)(`span`,{className:`forward-lowering-arrow`,children:`->`}),i.graph.nodes.slice(3).map((e,t)=>(0,E.jsxs)(`div`,{className:`forward-lowering-flow-tail`,children:[(0,E.jsxs)(`article`,{children:[(0,E.jsx)(`strong`,{children:e.id}),(0,E.jsx)(`span`,{children:e.detail})]}),t<2?(0,E.jsx)(`span`,{className:`forward-lowering-arrow`,children:`->`}):null]},e.id))]}),(0,E.jsx)(`div`,{className:`forward-lowering-edge-grid`,children:i.graph.edges.slice(0,3).map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`code`,{children:e.id}),(0,E.jsxs)(`span`,{children:[e.from,` -> `,e.to]}),(0,E.jsxs)(`strong`,{children:[`x `,zi(e.weight)]})]},e.id))})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`NeuralIR instruction stream`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`2 - schedule`}),(0,E.jsx)(`h2`,{children:`NeuralIR writes each value once`})]}),(0,E.jsxs)(`code`,{children:[i.neuralIr.magic,` v`,i.neuralIr.version]})]}),(0,E.jsx)(`div`,{className:`forward-lowering-instruction-lane`,children:i.neuralIr.instructions.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Open NeuralIR ${e.id}, ${e.op}`,"aria-pressed":n.lane===`neural`&&n.id===e.id,onClick:()=>r({lane:`neural`,id:e.id}),type:`button`,children:[(0,E.jsx)(`small`,{children:e.id}),(0,E.jsx)(`strong`,{children:e.op}),(0,E.jsx)(`code`,{children:e.output??`output boundary`}),(0,E.jsx)(`span`,{children:Bi(e)})]},e.id))})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-ir`,"aria-label":`MatrixIR operation stream`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`3 - fuse`}),(0,E.jsx)(`h2`,{children:`MatrixIR keeps columns together`})]}),(0,E.jsxs)(`code`,{children:[i.matrixIr.magic,` v`,i.matrixIr.version]})]}),(0,E.jsx)(`div`,{className:`forward-lowering-matrix-lane`,children:i.matrixIr.operations.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Open MatrixIR ${e.id}, ${e.op}`,"aria-pressed":n.lane===`matrix`&&n.id===e.id,onClick:()=>r({lane:`matrix`,id:e.id}),type:`button`,children:[(0,E.jsx)(`small`,{children:e.id}),(0,E.jsx)(`strong`,{children:e.op}),(0,E.jsx)(`code`,{children:e.output??`output boundary`}),(0,E.jsx)(`span`,{children:Vi(e)})]},e.id))})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-selection`,"aria-label":`Selected lowering detail`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`selected translation`}),(0,E.jsx)(`h2`,{children:a?.op??o?.op})]}),(0,E.jsx)(`code`,{children:n.id})]}),a===void 0?o===void 0?null:(0,E.jsxs)(`div`,{className:`forward-lowering-detail-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`fuses NeuralIR`}),(0,E.jsx)(`code`,{children:o.sourceInstructions.join(`, `)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`attributes`}),(0,E.jsx)(`code`,{children:M(o.attributes)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`graph provenance`}),(0,E.jsx)(`code`,{children:[...o.sourceNodes,...o.sourceEdges].join(`, `)||`none`})]})]}):(0,E.jsxs)(`div`,{className:`forward-lowering-detail-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`reads`}),(0,E.jsx)(`code`,{children:s?.reads.map(e=>`${e.valueId}=${zi(e.value)}`).join(`, `)||`none`})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`writes`}),(0,E.jsx)(`code`,{children:s?.write===void 0?`${s?.output?.outputName}=${zi(s?.output?.value??0)}`:`${s.write.valueId}=${zi(s.write.value)}`})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`graph provenance`}),(0,E.jsx)(`code`,{children:[...a.sourceNodes,...a.sourceEdges].join(`, `)||`none`})]})]})]}),(0,E.jsxs)(`section`,{className:`forward-lowering-parity`,"aria-label":`Forward lowering execution parity`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`4 - prove equivalence`}),(0,E.jsx)(`h2`,{children:`Three paths, the same prediction`})]}),(0,E.jsxs)(`code`,{children:[`max error `,i.maxParityError.toExponential(1)]})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-parity-table`,role:`table`,"aria-label":`Direct NeuralIR MatrixIR outputs`,children:[(0,E.jsxs)(`div`,{className:`forward-lowering-parity-head`,role:`row`,children:[(0,E.jsx)(`strong`,{role:`columnheader`,children:`row`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`x0`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`x1`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`direct`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`NeuralIR`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`MatrixIR`})]}),i.directOutputs.map((e,t)=>(0,E.jsxs)(`div`,{role:`row`,children:[(0,E.jsx)(`strong`,{role:`cell`,children:t}),(0,E.jsx)(`code`,{role:`cell`,children:zi(i.scenario.inputs.x0[t])}),(0,E.jsx)(`code`,{role:`cell`,children:zi(i.scenario.inputs.x1[t])}),(0,E.jsx)(`code`,{role:`cell`,children:zi(e)}),(0,E.jsx)(`code`,{role:`cell`,children:zi(i.neuralIrOutputs[t])}),(0,E.jsx)(`code`,{role:`cell`,children:zi(i.matrixIrOutputs[t])})]},t))]})]})]}),(0,E.jsxs)(`aside`,{className:`forward-lowering-controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Run shape`}),(0,E.jsx)(`h2`,{children:`Keep the compiler fixed`}),(0,E.jsx)(`p`,{children:`Change only the number of input rows and watch every IR identifier stay stable.`}),(0,E.jsx)(`div`,{className:`forward-lowering-scenario-buttons`,children:xi.map(n=>(0,E.jsxs)(`button`,{"aria-label":n.title,"aria-pressed":e===n.id,onClick:()=>t(n.id),type:`button`,children:[(0,E.jsx)(`strong`,{children:n.title}),(0,E.jsx)(`span`,{children:n.summary})]},n.id))}),(0,E.jsxs)(`div`,{className:`forward-lowering-equation`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Paper result`}),(0,E.jsx)(`code`,{children:`z = -1 + 0.25x0 + 0.75x1`}),(0,E.jsx)(`code`,{children:`prediction = max(0, z)`})]}),(0,E.jsxs)(`div`,{className:`forward-lowering-mental-model`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Rust boundary`}),(0,E.jsx)(`h2`,{children:`Meaning stays above tensors`}),(0,E.jsx)(`p`,{children:`The neural compiler retains source IDs and fusion rules. A Rust MX01 bridge receives explicit tensors, dtypes, shapes, constants, inputs, and outputs.`})]})]})]})}var Ui=/^[a-z][a-z0-9_]{0,31}$/,Wi=4,Gi=12,Ki=1e3,qi=0xe8d4a51000,Ji=1e-5,Yi=[{id:`a`,input:2,target:1},{id:`b`,input:-1,target:1}],Xi=[{id:`accumulate_two_calls`,title:`Two backward calls`,summary:`The second backward adds 2 to the 2 already in w.grad.`,initialParameter:1,learningRate:.1,samples:Yi,events:[{kind:`backward`,sampleId:`a`},{kind:`backward`,sampleId:`b`}]},{id:`zero_between_calls`,title:`Zero between calls`,summary:`Clearing the buffer makes the second gradient stand alone.`,initialParameter:1,learningRate:.1,samples:Yi,events:[{kind:`backward`,sampleId:`a`},{kind:`zero_grad`},{kind:`backward`,sampleId:`b`}]},{id:`mean_then_zero`,title:`Mean, step, zero`,summary:`Two micro-batches become one mean gradient, then zero starts the next batch clean.`,initialParameter:1,learningRate:.1,samples:Yi,events:[{kind:`backward`,sampleId:`a`},{kind:`backward`,sampleId:`b`},{kind:`optimizer_step`,divisor:2},{kind:`zero_grad`}]},{id:`stale_next_batch`,title:`Forgotten zero`,summary:`A new 0.8 gradient lands on a stale buffer of 4 and drives the wrong update.`,initialParameter:1,learningRate:.1,samples:[...Yi,{id:`c`,input:1,target:0}],events:[{kind:`backward`,sampleId:`a`},{kind:`backward`,sampleId:`b`},{kind:`optimizer_step`,divisor:2},{kind:`backward`,sampleId:`c`},{kind:`optimizer_step`,divisor:1}]}];function Zi(e,t){if(!Number.isFinite(e)||Math.abs(e)>qi)throw Error(`${t} must remain finite and bounded`);return e}function Qi(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>Ki)throw Error(`${t} must be finite and bounded`)}function $i(e){if(typeof e!=`object`||!e||!Array.isArray(e.samples)||!Array.isArray(e.events))throw Error(`gradient schedule must contain bounded sample and event arrays`);if(typeof e.id!=`string`||!Ui.test(e.id)||typeof e.title!=`string`||e.title.length<1||e.title.length>256||typeof e.summary!=`string`||e.summary.length<1||e.summary.length>512)throw Error(`gradient schedule metadata must contain bounded strings`);if(e.samples.length<1||e.samples.length>Wi||e.events.length<1||e.events.length>Gi)throw Error(`gradient schedule exceeds bounded sizes`);if(Qi(e.initialParameter,`initial parameter`),Qi(e.learningRate,`learning rate`),e.learningRate<=0||e.learningRate>1)throw Error(`learning rate must be in (0, 1]`);let t=new Set;e.samples.forEach(e=>{if(typeof e!=`object`||!e||typeof e.id!=`string`||!Ui.test(e.id))throw Error(`sample must have a bounded identifier`);if(t.has(e.id))throw Error(`duplicate sample id ${e.id}`);Qi(e.input,`sample ${e.id} input`),Qi(e.target,`sample ${e.id} target`),t.add(e.id)});let n=0;if(e.events.forEach(e=>{if(typeof e!=`object`||!e)throw Error(`event must be an object`);if(e.kind===`backward`){if(typeof e.sampleId!=`string`||!Ui.test(e.sampleId))throw Error(`backward sample id must be a bounded identifier`);if(!t.has(e.sampleId))throw Error(`backward references unknown sample ${e.sampleId}`);n+=1}else if(e.kind===`optimizer_step`){if(!Number.isInteger(e.divisor)||e.divisor<1||e.divisor>Wi)throw Error(`optimizer divisor must be a bounded positive integer`)}else if(e.kind!==`zero_grad`)throw Error(`unsupported gradient schedule event`)}),n===0)throw Error(`gradient schedule needs a backward call`)}function ea(e){let t=e.samples.map(e=>Object.freeze({...e})),n=e.events.map(e=>Object.freeze({...e}));return Object.freeze({id:e.id,title:e.title,summary:e.summary,initialParameter:e.initialParameter,learningRate:e.learningRate,samples:Object.freeze(t),events:Object.freeze(n)})}function ta(e,t){let n=Zi(Zi(e*t.input,`finite-difference prediction`)-t.target,`finite-difference residual`);return Zi(.5*n*n,`finite-difference loss`)}function na(e,t=Ji){if($i(e),!Number.isFinite(t)||t<1e-12||t>1)throw Error(`finite-difference epsilon must be in [1e-12, 1]`);let n=ea(e),r=new Map(n.samples.map(e=>[e.id,e])),i=[],a=n.initialParameter,o=0,s=0,c=0,l=0,u=0;return n.events.forEach((e,d)=>{let f=a,p=o;if(e.kind===`backward`){let n=r.get(e.sampleId),c=Zi(a*n.input,`event ${d} prediction`),l=Zi(c-n.target,`event ${d} residual`),m=Zi(.5*l*l,`event ${d} loss`),h=Zi(l*n.input,`event ${d} gradient`);o=Zi(o+h,`event ${d} buffer`);let g=Zi((ta(a+t,n)-ta(a-t,n))/(2*t),`event ${d} numerical gradient`),_=Math.abs(h-g);u=Math.max(u,_),s+=1,i.push({index:d,kind:`backward`,sampleId:n.id,input:n.input,target:n.target,parameterBefore:f,parameterAfter:a,bufferBefore:p,bufferAfter:o,prediction:c,residual:l,loss:m,localGradient:h,numericalGradient:g,gradientAbsoluteError:_})}else if(e.kind===`zero_grad`)o=0,l+=1,i.push({index:d,kind:`zero_grad`,parameterBefore:f,parameterAfter:a,bufferBefore:p,bufferAfter:o});else{let t=Zi(o/e.divisor,`event ${d} applied gradient`),r=Zi(-n.learningRate*t,`event ${d} parameter delta`);a=Zi(a+r,`event ${d} parameter`),c+=1,i.push({index:d,kind:`optimizer_step`,parameterBefore:f,parameterAfter:a,bufferBefore:p,bufferAfter:o,divisor:e.divisor,appliedGradient:t,parameterDelta:r})}}),{scenario:n,steps:i,finalParameter:a,finalGradientBuffer:o,backwardCalls:s,optimizerSteps:c,zeroCalls:l,maxGradientAbsoluteError:u}}function ra(e){let t=Xi.find(t=>t.id===e);if(!t)throw Error(`unknown gradient accumulation scenario: ${e}`);return na(t)}function N(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function ia(e){return e.kind===`backward`?`backward(${e.sampleId})`:e.kind===`zero_grad`?`zero_grad()`:`step(grad / ${e.divisor})`}function aa(){let[e,t]=(0,l.useState)(`accumulate_two_calls`),[n,r]=(0,l.useState)(0),i=(0,l.useMemo)(()=>ra(e),[e]),a=i.steps[Math.min(n,i.steps.length-1)];function o(e){t(e),r(0)}return(0,E.jsxs)(`main`,{className:`workspace workspace--gradient-buffer`,children:[(0,E.jsxs)(`section`,{className:`gradient-buffer-stage`,"aria-label":`Gradient accumulation and zeroing visualizer`,children:[(0,E.jsxs)(`section`,{className:`gradient-buffer-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN28 / tensor and autograd bridge`}),(0,E.jsx)(`h2`,{children:`Gradient buffer timeline`}),(0,E.jsx)(`p`,{children:`Backward adds into a persistent buffer. An optimizer reads it, but only an explicit zero clears it.`})]}),(0,E.jsx)(`div`,{className:`gradient-buffer-chip`,children:`w.grad += local`})]}),(0,E.jsxs)(`section`,{className:`gradient-buffer-state`,"aria-label":`Selected gradient buffer state`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`The two pieces of mutable state`}),(0,E.jsx)(`h2`,{children:`Parameter and gradient buffer`})]}),(0,E.jsxs)(`span`,{children:[`event `,a.index+1,` of `,i.steps.length]})]}),(0,E.jsxs)(`div`,{className:`gradient-buffer-vessels`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`parameter w`}),(0,E.jsx)(`code`,{children:N(a.parameterBefore)}),(0,E.jsx)(`span`,{children:`→`}),(0,E.jsx)(`strong`,{children:N(a.parameterAfter)})]}),(0,E.jsxs)(`div`,{className:a.bufferAfter===0?`is-empty`:`is-filled`,children:[(0,E.jsx)(`small`,{children:`persistent w.grad`}),(0,E.jsx)(`code`,{children:N(a.bufferBefore)}),(0,E.jsx)(`span`,{children:`→`}),(0,E.jsx)(`strong`,{children:N(a.bufferAfter)})]})]})]}),(0,E.jsxs)(`section`,{className:`gradient-buffer-timeline`,"aria-label":`Gradient schedule timeline`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Replay the schedule`}),(0,E.jsx)(`h2`,{children:`Every API call is a state transition`})]}),(0,E.jsxs)(`span`,{children:[i.backwardCalls,` backward / `,i.optimizerSteps,` step / `,i.zeroCalls,` zero`]})]}),(0,E.jsx)(`div`,{className:`gradient-buffer-event-lane`,children:i.steps.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Open event ${e.index+1}, ${ia(e)}, buffer ${N(e.bufferBefore)} to ${N(e.bufferAfter)}`,"aria-pressed":e.index===a.index,type:`button`,onClick:()=>r(e.index),children:[(0,E.jsxs)(`small`,{children:[`event `,e.index+1]}),(0,E.jsx)(`strong`,{children:ia(e)}),(0,E.jsxs)(`code`,{children:[`grad `,N(e.bufferBefore),` → `,N(e.bufferAfter)]})]},e.index))})]}),(0,E.jsxs)(`section`,{className:`gradient-buffer-equation`,"aria-label":`Selected gradient buffer calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Open the arithmetic`}),(0,E.jsx)(`h2`,{children:ia(a)})]}),(0,E.jsx)(`span`,{children:a.kind.replace(`_`,` `)})]}),a.kind===`backward`?(0,E.jsxs)(`div`,{className:`gradient-buffer-backward-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`forward sample `,a.sampleId]}),(0,E.jsxs)(`code`,{children:[N(a.parameterBefore),` × `,N(a.input),` = `,N(a.prediction)]}),(0,E.jsxs)(`code`,{children:[N(a.prediction),` - `,N(a.target),` = `,N(a.residual)]}),(0,E.jsxs)(`strong`,{children:[`½ × `,N(a.residual),`² = `,N(a.loss)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`local gradient`}),(0,E.jsxs)(`code`,{children:[`(`,N(a.prediction),` - `,N(a.target),`) × `,N(a.input)]}),(0,E.jsxs)(`strong`,{children:[`dL/dw = `,N(a.localGradient)]})]}),(0,E.jsxs)(`div`,{className:`gradient-buffer-addition`,children:[(0,E.jsx)(`small`,{children:`buffer addition`}),(0,E.jsxs)(`code`,{children:[N(a.bufferBefore),` + `,N(a.localGradient)]}),(0,E.jsxs)(`strong`,{children:[`w.grad = `,N(a.bufferAfter)]})]})]}):a.kind===`zero_grad`?(0,E.jsxs)(`div`,{className:`gradient-buffer-zero-rule`,children:[(0,E.jsx)(`code`,{children:`w.grad ← 0`}),(0,E.jsxs)(`p`,{children:[`The parameter stays `,N(a.parameterAfter),`. Only the buffer is cleared.`]})]}):(0,E.jsxs)(`div`,{className:`gradient-buffer-step-rule`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`choose sum or mean`}),(0,E.jsxs)(`code`,{children:[N(a.bufferBefore),` / `,a.divisor,` = `,N(a.appliedGradient)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`SGD update`}),(0,E.jsxs)(`code`,{children:[N(a.parameterBefore),` - `,N(i.scenario.learningRate),` × `,N(a.appliedGradient)]}),(0,E.jsxs)(`strong`,{children:[`w = `,N(a.parameterAfter)]})]}),(0,E.jsxs)(`p`,{children:[`The optimizer read `,N(a.bufferBefore),` but left that buffer unchanged.`]})]})]}),(0,E.jsxs)(`section`,{className:`gradient-buffer-audit`,"aria-label":`Gradient buffer numerical audit`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Independent check`}),(0,E.jsx)(`h2`,{children:`Each local gradient gets fresh forward passes`})]}),(0,E.jsx)(`span`,{children:`epsilon 1e-5`})]}),(0,E.jsxs)(`div`,{className:`gradient-buffer-audit-grid`,children:[i.steps.filter(e=>e.kind===`backward`).map(e=>e.kind===`backward`?(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`strong`,{children:[`event `,e.index+1,` / `,e.sampleId]}),(0,E.jsxs)(`span`,{children:[`analytical `,(0,E.jsx)(`code`,{children:N(e.localGradient)})]}),(0,E.jsxs)(`span`,{children:[`numerical `,(0,E.jsx)(`code`,{children:N(e.numericalGradient)})]}),(0,E.jsxs)(`small`,{children:[`error `,N(e.gradientAbsoluteError)]})]},e.index):null),(0,E.jsxs)(`div`,{className:`gradient-buffer-audit-max`,children:[(0,E.jsx)(`strong`,{children:`maximum error`}),(0,E.jsx)(`code`,{children:N(i.maxGradientAbsoluteError)}),(0,E.jsx)(`small`,{children:`must stay below 1e-8`})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`controls gradient-buffer-controls`,"aria-label":`Gradient buffer scenarios`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Schedule presets`}),(0,E.jsx)(`h2`,{children:`Move the zero call`}),(0,E.jsx)(`div`,{className:`gradient-buffer-scenario-buttons`,children:Xi.map(t=>(0,E.jsxs)(`button`,{"aria-pressed":t.id===e,type:`button`,onClick:()=>o(t.id),children:[(0,E.jsx)(`strong`,{children:t.title}),(0,E.jsx)(`span`,{children:t.summary})]},t.id))}),(0,E.jsxs)(`div`,{className:`gradient-buffer-summary`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Final state`}),(0,E.jsxs)(`code`,{children:[`w = `,N(i.finalParameter)]}),(0,E.jsxs)(`code`,{children:[`w.grad = `,N(i.finalGradientBuffer)]})]}),(0,E.jsxs)(`div`,{className:`gradient-buffer-mental-model`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Keep this picture`}),(0,E.jsx)(`h2`,{children:`Backward adds. Step reads. Zero clears.`}),(0,E.jsx)(`p`,{children:`Accumulation is useful across micro-batches and dangerous across optimizer steps unless the schedule is deliberate.`})]})]})]})}function oa(e){return e.map(e=>[...e])}function sa(e){return{name:e.name,weights:oa(e.weights),biases:[...e.biases],activation:e.activation}}function ca(e,t){if(t.length===0||t[0]===void 0||t[0].length===0)throw Error(`${e} must have at least one row and one column`);let n=t[0].length;for(let r of t)if(r.length!==n)throw Error(`${e} must be rectangular`);return{rows:t.length,cols:n}}function la(e){let t=e>>>0;return()=>(t=Math.imul(1664525,t)+1013904223>>>0,t/4294967296)}function ua(e,t){return(e()*2-1)*t}function da(e,t){return e.map(e=>e.map((e,n)=>e+t[n]))}function fa(e){let t=e[0]?.length??0,n=Array(t).fill(0);for(let r of e)for(let e=0;e<t;e+=1)n[e]+=r[e];return n}function pa(e){let t=0,n=0;for(let r of e)for(let e of r)t+=e*e,n+=1;return n===0?0:t/n}function ma(e){return e.length===0?0:e.reduce((e,t)=>e+t*t,0)/e.length}function ha(e,t){switch(t){case`linear`:return e;case`sigmoid`:if(e>=0)return 1/(1+Math.exp(-e));{let t=Math.exp(e);return t/(1+t)}case`tanh`:return Math.tanh(e);case`relu`:return Math.max(0,e)}}function ga(e,t,n){switch(n){case`linear`:return 1;case`sigmoid`:return t*(1-t);case`tanh`:return 1-t*t;case`relu`:return+(e>0)}}function _a(e,t){return e.map(e=>e.map(e=>ha(e,t)))}function va(e,t,n){return e.map((e,r)=>e.map((e,i)=>ga(e,t[r][i],n)))}function ya(e,t){return e.map((e,n)=>e.map((e,r)=>e*t[n][r]))}function ba(e,t,n){return new bt(e).subtract(new bt(t).scale(n)).data}function xa(e,t,n){return e.map((e,r)=>e-n*t[r])}function Sa(e,t,n){let r=ca(`${e.name} weights`,e.weights);if(r.rows!==t)throw Error(`${e.name} weight row count must match previous layer width`);if(r.cols!==e.biases.length)throw Error(`${e.name} weight columns must match bias count`);if(e.biases.length===0)throw Error(`${e.name} must have at least one neuron`);for(let t of e.weights)for(let n of t)if(!Number.isFinite(n))throw Error(`${e.name} weights must be finite`);for(let t of e.biases)if(!Number.isFinite(t))throw Error(`${e.name} biases must be finite`);if(n<0)throw Error(`layer index must be non-negative`)}function Ca(e,t,n,r,i=7,a=1){if(n<1)throw Error(`hiddenLayerCount must be at least one`);let o=la(i),s=[],c=e;for(let e=0;e<n;e+=1){let r=n===1?a:a/Math.sqrt(Math.max(1,c));s.push({name:`hidden${e+1}`,weights:Array.from({length:c},()=>Array.from({length:t},()=>ua(o,r))),biases:Array.from({length:t},()=>ua(o,r)),activation:`sigmoid`}),c=t}let l=n===1?a:a/Math.sqrt(Math.max(1,c));return s.push({name:`output`,weights:Array.from({length:c},()=>Array.from({length:r},()=>ua(o,l))),biases:Array.from({length:r},()=>ua(o,l)),activation:`sigmoid`}),{layers:s}}function wa(e,t){let n=ca(`inputs`,e);if(t.layers.length===0)throw Error(`layered network must have at least one layer`);let r=[],i=[],a=e,o=n.cols;for(let[e,n]of t.layers.entries()){Sa(n,o,e);let t=da(new bt(a).dot(new bt(n.weights)).data,n.biases),s=_a(t,n.activation);r.push(t),i.push(s),a=s,o=n.biases.length}return{rawByLayer:r,activationsByLayer:i,predictions:i[i.length-1]}}function Ta(e,t,n,r){let i=ca(`inputs`,e),a=ca(`targets`,t);if(i.rows!==a.rows)throw Error(`inputs and targets must have the same number of rows`);let o=wa(e,n);if(ca(`predictions`,o.predictions).cols!==a.cols)throw Error(`prediction width must match target width`);let s=new bt(o.predictions).subtract(new bt(t)).data,c=2/(a.rows*a.cols),l=s.map(e=>e.map(e=>c*e)),u=Array(n.layers.length),d=n.layers.length-1;u[d]=ya(l,va(o.rawByLayer[d],o.activationsByLayer[d],n.layers[d].activation));for(let e=d-1;e>=0;--e){let t=new bt(u[e+1]).dot(new bt(n.layers[e+1].weights).transpose()).data;u[e]=ya(t,va(o.rawByLayer[e],o.activationsByLayer[e],n.layers[e].activation))}let f=[],p=[],m=n.layers.map((t,n)=>{let i=n===0?e:o.activationsByLayer[n-1],a=u[n],s=new bt(i).transpose().dot(new bt(a)).data,c=fa(a);return f.push(s),p.push(c),{...sa(t),weights:ba(t.weights,s,r),biases:xa(t.biases,c,r)}});return{...o,errors:s,deltas:u,weightGradients:f,biasGradients:p,nextParameters:{layers:m},loss:pa(s)}}function Ea(e,t,n=0,r){let i=wa(e,t);if(n<0||n>=e.length)throw Error(`exampleIndex must refer to an input row`);let a=e[n],o=i.predictions[n];if(r!==void 0&&r.length!==o.length)throw Error(`target width must match prediction width`);let s,c,l;r!==void 0&&(c=o.map((e,t)=>e-r[t]),l=ma(c),s=Ta([a],[r],t,0).deltas);let u=t.layers.map((e,r)=>{let o=r===0?a:i.activationsByLayer[r-1][n],c=r===0?`input`:t.layers[r-1].name,l=i.rawByLayer[r][n],u=i.activationsByLayer[r][n].map((t,n)=>({neuron:`${e.name}[${n}]`,incoming:o.map((t,r)=>{let i=e.weights[r][n];return{source:`${c}[${r}]`,value:t,weight:i,contribution:t*i}}),bias:e.biases[n],rawSum:l[n],activation:e.activation,output:t,delta:s?.[r]?.[0]?.[n]}));return{layer:e.name,neurons:u}});return{exampleIndex:n,inputs:[...a],target:r===void 0?void 0:[...r],prediction:[...o],error:c,loss:l,layers:u}}function Da(e){return Math.max(0,e.layers.length-1)}function Oa(e){return e===void 0||e.length===0?`0x0`:`${e.length}x${e[0]?.length??0}`}function ka(e,t){let n=st(Ze({name:`ml-learning-linear-visualizer`,inputNames:[`x`],layers:[{name:`output`,weights:[[t.weight]],biases:[t.bias],activation:`none`,outputNames:[`prediction`]}]})),r=Ct(n);return{predictions:Tt(r,{x:e}).outputs.prediction??[],bytecodeInstructionCount:n.functions[0]?.instructions.length??0,matrixInstructionCount:r.instructions.length}}function Aa(e,t,n={}){let r=Pa(e,t,n),i=Tt(r.matrixPlan,r.matrixInputs);return{predictions:Fa(e,r.outputNames,i.outputs),bytecodeInstructionCount:r.bytecodeInstructionCount,matrixInstructionCount:r.matrixInstructionCount}}async function ja(e,t,n={}){let r=Pa(e,t,n),i=await Ia();if(i.backend!==null){let t=await wt(r.matrixPlan,r.matrixInputs,i.backend);return{predictions:Fa(e,r.outputNames,t.outputs),bytecodeInstructionCount:r.bytecodeInstructionCount,matrixInstructionCount:r.matrixInstructionCount,backend:`webgpu`}}let a=Tt(r.matrixPlan,r.matrixInputs);return{predictions:Fa(e,r.outputNames,a.outputs),bytecodeInstructionCount:r.bytecodeInstructionCount,matrixInstructionCount:r.matrixInstructionCount,backend:`cpu`,fallbackReason:i.reason}}function Ma(){return zt.isNavigatorAvailable()}var Na;function Pa(e,t,n){let r=t.layers[0],i=t.layers[t.layers.length-1];if(r===void 0||i===void 0)throw Error(`layered VM prediction requires at least one layer`);let a=e[0]?.length??r.weights.length,o=i.biases.length,s=n.inputNames??Array.from({length:a},(e,t)=>`input${t}`),c=n.outputNames??Array.from({length:o},(e,t)=>o===1?`prediction`:`output${t}`),l=st(Ze({name:`ml-learning-layered-visualizer`,inputNames:s,layers:t.layers.map((e,n)=>({name:e.name,weights:e.weights,biases:e.biases,activation:Ra(e.activation),outputNames:n===t.layers.length-1?c:void 0}))})),u=Ct(l);return{matrixPlan:u,matrixInputs:Object.fromEntries(s.map((t,n)=>[t,e.map(e=>e[n]??0)])),outputNames:c,bytecodeInstructionCount:l.functions[0]?.instructions.length??0,matrixInstructionCount:u.instructions.length}}function Fa(e,t,n){return e.map((e,r)=>t.map(e=>n[e]?.[r]??0))}async function Ia(){return Na??=La(),Na}async function La(){if(!zt.isNavigatorAvailable())return{backend:null,reason:`WebGPU is not exposed by this browser`};try{let e=await zt.createFromNavigator({powerPreference:`high-performance`});return{backend:e,reason:e===null?`WebGPU is not exposed by this browser`:void 0}}catch(e){return{backend:null,reason:e instanceof Error?e.message:`WebGPU initialization failed`}}}function Ra(e){return e===`linear`?`none`:e}function za(e){return{...e,defaultHiddenLayerCount:e.defaultHiddenLayerCount??1,hiddenLayerMin:e.hiddenLayerMin??1,hiddenLayerMax:e.hiddenLayerMax??4,learningRateMin:e.defaultLearningRate/20,learningRateMax:e.defaultLearningRate*8,learningRateStep:e.defaultLearningRate/20}}function Ba(e){return e.rows.map(e=>[e.target])}function Va(e){return e.rows.map(e=>e.input)}function Ha(e,t=e.defaultHiddenLayerCount){let n=Math.max(e.hiddenLayerMin,Math.min(e.hiddenLayerMax,Math.round(t)));return{epoch:0,hiddenLayerCount:n,parameters:Ca(e.inputLabels.length,e.hiddenCount,n,1,e.seed,e.initialScale)}}function Ua(e,t){return Aa(Va(e),t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}).predictions.map(e=>e[0])}function Wa(e,t){let n=Ua(e,t);return n.reduce((t,n,r)=>{let i=n-e.rows[r].target;return t+i*i},0)/n.length}function Ga(e,t){let n=Ua(e,t);return n.reduce((t,n,r)=>t+Math.abs(n-e.rows[r].target),0)/n.length}function Ka(e,t){return{epoch:t.epoch,loss:Wa(e,t),mae:Ga(e,t)}}function qa(e,t,n){let r=Ta(Va(e),Ba(e),t.parameters,n),i={epoch:t.epoch+1,hiddenLayerCount:Da(r.nextParameters),parameters:r.nextParameters};return{previousState:t,state:i,step:r,loss:Wa(e,i),mae:Ga(e,i)}}function Ja(e,t,n,r){let i=[],a=t;for(let t=0;t<r;t+=1){let t=qa(e,a,n);i.push(t),a=t.state}return i}function Ya(e,t,n){return{...Ea([e.rows[n].input],t.parameters,0,[e.rows[n].target]),exampleIndex:n}}var Xa=[];for(let e of[-1,-.5,0,.5,1])for(let t of[-1,-.5,0,.5,1])Xa.push({input:[e,t],target:+(e*e+t*t<=.55),label:`(${e}, ${t})`,group:e*e+t*t<=.55?`inside`:`outside`});var Za=[];for(let e=0;e<12;e+=1){let t=Math.PI*e/11;Za.push({input:[Math.cos(t),Math.sin(t)],target:0,label:`upper ${e+1}`,group:`upper`}),Za.push({input:[1-Math.cos(t),.5-Math.sin(t)],target:1,label:`lower ${e+1}`,group:`lower`})}var Qa=[za({id:`xnor`,title:`XNOR Gate`,category:`Logic`,summary:`Outputs 1 when the two inputs match and 0 when they differ.`,lesson:`The hidden layer learns two useful regions: both inputs off and both inputs on. The output neuron combines those regions into one decision.`,inputLabels:[`A`,`B`],outputLabel:`same?`,rows:[{input:[0,0],target:1,label:`A=0, B=0`,group:`same`},{input:[0,1],target:0,label:`A=0, B=1`,group:`different`},{input:[1,0],target:0,label:`A=1, B=0`,group:`different`},{input:[1,1],target:1,label:`A=1, B=1`,group:`same`}],hiddenCount:3,initialScale:2,seed:31,defaultLearningRate:1.4,chartKind:`surface`}),za({id:`absolute-value`,title:`Absolute Value`,category:`Regression`,summary:`Learns the V-shaped relationship y = |x| on normalized inputs.`,lesson:`A single line cannot bend at zero. Hidden neurons can split the input range into left and right regions, then recombine them into a V.`,inputLabels:[`x`],outputLabel:`|x|`,rows:[-1,-.75,-.5,-.25,0,.25,.5,.75,1].map(e=>({input:[e],target:Math.abs(e),label:`x=${e}`})),hiddenCount:6,initialScale:3,seed:12,defaultLearningRate:1.8,chartKind:`curve`}),za({id:`piecewise-pricing`,title:`Piecewise Pricing`,category:`Regression`,summary:`Approximates a stepped shipping-price schedule from package weight.`,lesson:`Hidden neurons can behave like soft thresholds. Several thresholds together make a stair-step curve.`,inputLabels:[`weight`],outputLabel:`price tier`,rows:[[.05,.12],[.15,.12],[.25,.25],[.35,.25],[.45,.55],[.55,.55],[.7,.88],[.85,.88],[1,.88]].map(([e,t])=>({input:[e],target:t,label:`${Math.round(e*40)} lb`})),hiddenCount:6,initialScale:3,seed:19,defaultLearningRate:2,chartKind:`curve`}),za({id:`circle-classifier`,title:`Circle Classifier`,category:`Classification`,summary:`Classifies whether a point is inside a circle.`,lesson:`The hidden layer combines several soft boundaries. Together they can carve out a round-ish region even though each neuron is simple.`,inputLabels:[`x`,`y`],outputLabel:`inside?`,rows:Xa,hiddenCount:8,initialScale:3,seed:37,defaultLearningRate:2.2,chartKind:`surface`}),za({id:`two-moons`,title:`Two Moons`,category:`Classification`,summary:`Separates two curved bands that no single straight boundary can split.`,lesson:`The hidden layer remaps curved geometry into features the output neuron can combine into a useful decision.`,inputLabels:[`x`,`y`],outputLabel:`moon`,rows:Za,hiddenCount:10,initialScale:3,seed:43,defaultLearningRate:1.8,chartKind:`surface`}),za({id:`interaction-features`,title:`Interaction Features`,category:`Tabular`,summary:`Predicts a normalized house-value score from bedrooms, bathrooms, and garage.`,lesson:`The hidden layer can learn combinations, like garage plus enough rooms, instead of treating each input as a separate straight-line effect.`,inputLabels:[`bedrooms`,`bathrooms`,`garage`],outputLabel:`value score`,rows:[{input:[.2,.25,0],target:.08,label:`1 bed, 1 bath, no garage`},{input:[.4,.25,0],target:.18,label:`2 bed, 1 bath, no garage`},{input:[.4,.5,0],target:.32,label:`2 bed, 2 bath, no garage`},{input:[.6,.5,0],target:.45,label:`3 bed, 2 bath, no garage`},{input:[.6,.5,1],target:.72,label:`3 bed, 2 bath, garage`},{input:[.8,.5,0],target:.58,label:`4 bed, 2 bath, no garage`},{input:[.8,.75,1],target:.9,label:`4 bed, 3 bath, garage`},{input:[1,.75,1],target:.96,label:`5 bed, 3 bath, garage`},{input:[1,1,0],target:.76,label:`5 bed, 4 bath, no garage`},{input:[.2,.5,1],target:.35,label:`1 bed, 2 bath, garage`}],hiddenCount:7,initialScale:3,seed:51,defaultLearningRate:1.8,chartKind:`table`})],$a=class extends Error{kind;constructor(e){super(`No handler registered for instruction kind: '${e}'`),this.name=`UnknownInstructionError`,this.kind=e}},eo=class extends Error{kind;constructor(e){super(`Handler already registered for instruction kind: '${e}'`),this.name=`DuplicateHandlerError`,this.kind=e}},to=class extends Error{constructor(e){super(`export() is not supported by the ${e} backend. Use a backend that supports pixel readback (Canvas, Metal, Cairo).`),this.name=`ExportNotSupportedError`}},no=class extends Error{constructor(){super(`execute() and patch() require a non-null context`),this.name=`NullContextError`}},ro=class{table=new Map;clearFn;exportFn;constructor(e,t){this.clearFn=e,this.exportFn=t}register(e,t){if(this.table.has(e))throw new eo(e);this.table.set(e,t)}dispatch(e,t){let n=this.table.get(e.kind)??this.table.get(`*`);if(!n)throw new $a(e.kind);n(e,t,this)}execute(e,t){if(t==null)throw new no;this.clearFn(t,e.background,e.width,e.height);for(let n of e.instructions)this.dispatch(n,t)}patch(e,t,n,r){if(n==null)throw new no;if(!r){this.execute(t,n);return}let{onDelete:i,onInsert:a,onUpdate:o}=r,s=new Map,c=new Map;for(let t of e.instructions)t.id&&s.set(t.id,t);for(let e of t.instructions)e.id&&c.set(e.id,e);for(let[e,t]of s)c.has(e)||i?.(t);for(let n=0;n<t.instructions.length;n++){let r=t.instructions[n];if(r.id&&s.has(r.id)){let e=s.get(r.id);io(r,e)||o?.(e,r)}else if(n<e.instructions.length){let t=e.instructions[n];io(r,t)||o?.(t,r)}else a?.(r,n)}}export(e,t){if(!this.exportFn)throw new to(`this`);let n={scale:t?.scale??1,channels:t?.channels??4,bit_depth:t?.bit_depth??8,color_space:t?.color_space??`srgb`};return this.exportFn(e,this,n)}registeredKinds(){return[...this.table.keys()]}};function io(e,t){if(e===t)return!0;if(e==null||t==null||typeof e!=typeof t)return!1;if(typeof e!=`object`)return e===t;if(Array.isArray(e)!==Array.isArray(t))return!1;if(Array.isArray(e)&&Array.isArray(t)){if(e.length!==t.length)return!1;for(let n=0;n<e.length;n++)if(!io(e[n],t[n]))return!1;return!0}let n=e,r=t,i=Object.keys(n),a=Object.keys(r);if(i.length!==a.length)return!1;for(let e of i)if(!Object.prototype.hasOwnProperty.call(r,e)||!io(n[e],r[e]))return!1;return!0}function ao(){return{defs:[],elements:[],clipCounter:0,filterCounter:0}}function oo(e){return e.replace(/&/g,`&amp;`).replace(/"/g,`&quot;`).replace(/</g,`&lt;`).replace(/>/g,`&gt;`)}function so(e){return e.replace(/&/g,`&amp;`).replace(/</g,`&lt;`).replace(/>/g,`&gt;`)}function co(e){let t=[];return t.push(`fill="${oo(e.fill??`none`)}"`),e.stroke&&(t.push(`stroke="${oo(e.stroke)}"`),t.push(`stroke-width="${P(e.stroke_width??1,`stroke_width`)}"`)),e.opacity!==void 0&&e.opacity!==1&&t.push(`opacity="${P(e.opacity,`opacity`)}"`),t.join(` `)}function lo(e){return e?` id="${oo(e)}"`:``}function uo(e){let t=e=>+e.toFixed(4);return e.map(e=>{switch(e.kind){case`move_to`:return`M ${t(e.x)} ${t(e.y)}`;case`line_to`:return`L ${t(e.x)} ${t(e.y)}`;case`quad_to`:return`Q ${t(e.cx)} ${t(e.cy)} ${t(e.x)} ${t(e.y)}`;case`cubic_to`:return`C ${t(e.cx1)} ${t(e.cy1)} ${t(e.cx2)} ${t(e.cy2)} ${t(e.x)} ${t(e.y)}`;case`arc_to`:return`A ${t(e.rx)} ${t(e.ry)} ${t(e.x_rotation)} ${+!!e.large_arc} ${+!!e.sweep} ${t(e.x)} ${t(e.y)}`;case`close`:return`Z`}}).join(` `)}function fo(e){if(!e)return``;let[t,n,r,i,a,o]=e;return` transform="matrix(${P(t,`transform.a`)},${P(n,`transform.b`)},${P(r,`transform.c`)},${P(i,`transform.d`)},${P(a,`transform.e`)},${P(o,`transform.f`)})"`}function P(e,t){if(!Number.isFinite(e))throw RangeError(`PaintVM SVG: ${t} must be a finite number, got ${e}`);return String(e)}function po(e,t){if(!t||t.length===0)return``;let n=[],r=`SourceGraphic`;for(let e=0;e<t.length;e++){let i=t[e],a=`f${e}`;switch(i.kind){case`blur`:n.push(`<feGaussianBlur in="${r}" stdDeviation="${P(i.radius,`blur.radius`)}" result="${a}"/>`);break;case`drop_shadow`:n.push(`<feDropShadow dx="${P(i.dx,`drop_shadow.dx`)}" dy="${P(i.dy,`drop_shadow.dy`)}" stdDeviation="${P(i.blur,`drop_shadow.blur`)}" flood-color="${oo(i.color)}" result="${a}"/>`);break;case`color_matrix`:{let e=i.matrix.map((e,t)=>P(e,`color_matrix.matrix[${t}]`));n.push(`<feColorMatrix in="${r}" type="matrix" values="${e.join(` `)}" result="${a}"/>`);break}case`brightness`:{let e=P(i.amount,`brightness.amount`);n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncR type="linear" slope="${e}"/><feFuncG type="linear" slope="${e}"/><feFuncB type="linear" slope="${e}"/></feComponentTransfer>`)}break;case`contrast`:{let e=i.amount,t=-(i.amount-1)/2;n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncR type="linear" slope="${P(e,`contrast.amount`)}" intercept="${P(t,`contrast.intercept`)}"/><feFuncG type="linear" slope="${P(e,`contrast.amount`)}" intercept="${P(t,`contrast.intercept`)}"/><feFuncB type="linear" slope="${P(e,`contrast.amount`)}" intercept="${P(t,`contrast.intercept`)}"/></feComponentTransfer>`)}break;case`saturate`:n.push(`<feColorMatrix in="${r}" type="saturate" values="${P(i.amount,`saturate.amount`)}" result="${a}"/>`);break;case`hue_rotate`:n.push(`<feColorMatrix in="${r}" type="hueRotate" values="${P(i.angle,`hue_rotate.angle`)}" result="${a}"/>`);break;case`invert`:{let e=P(i.amount,`invert.amount`),t=P(-i.amount,`invert.neg_amount`);n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncR type="linear" slope="${t}" intercept="${e}"/><feFuncG type="linear" slope="${t}" intercept="${e}"/><feFuncB type="linear" slope="${t}" intercept="${e}"/></feComponentTransfer>`)}break;case`opacity`:n.push(`<feComponentTransfer in="${r}" result="${a}"><feFuncA type="linear" slope="${P(i.amount,`opacity.amount`)}"/></feComponentTransfer>`);break}r=a}return`<filter id="${oo(e)}">${n.join(``)}</filter>`}var mo=new Set([`normal`,`multiply`,`screen`,`overlay`,`darken`,`lighten`,`color-dodge`,`color-burn`,`hard-light`,`soft-light`,`difference`,`exclusion`,`hue`,`saturation`,`color`,`luminosity`]);function ho(e){let t=e.replace(/_/g,`-`);return mo.has(t)?t:`normal`}function go(e,t){let n=co(e),r=e.corner_radius===void 0?``:` rx="${P(e.corner_radius,`rect.corner_radius`)}"`;t.elements.push(`<rect${lo(e.id)} x="${P(e.x,`rect.x`)}" y="${P(e.y,`rect.y`)}" width="${P(e.width,`rect.width`)}" height="${P(e.height,`rect.height`)}"${r} ${n}/>`)}function _o(e,t){let n=co(e);t.elements.push(`<ellipse${lo(e.id)} cx="${P(e.cx,`ellipse.cx`)}" cy="${P(e.cy,`ellipse.cy`)}" rx="${P(e.rx,`ellipse.rx`)}" ry="${P(e.ry,`ellipse.ry`)}" ${n}/>`)}var vo=new Set([`nonzero`,`evenodd`]),F=new Set([`butt`,`round`,`square`]),yo=new Set([`miter`,`round`,`bevel`]);function bo(e,t){let n=uo(e.commands),r=e.fill_rule&&vo.has(e.fill_rule)?e.fill_rule:`nonzero`,i=r===`nonzero`?``:` fill-rule="${r}"`,a=e.stroke_cap&&F.has(e.stroke_cap)?` stroke-linecap="${e.stroke_cap}"`:``,o=e.stroke_join&&yo.has(e.stroke_join)?` stroke-linejoin="${e.stroke_join}"`:``,s=co(e);t.elements.push(`<path${lo(e.id)} d="${oo(n)}"${i}${a}${o} ${s}/>`)}function xo(e,t){let n=e.fill??`#000000`,r=e.glyphs.map(e=>{let t=e.glyph_id,n=Number.isInteger(t)&&t>=0&&t<=1114111?t:65533;return`<tspan x="${P(e.x,`glyph.x`)}" y="${P(e.y,`glyph.y`)}">&#${n};</tspan>`});t.elements.push(`<text${lo(e.id)} font-size="${P(e.font_size,`glyph_run.font_size`)}" fill="${oo(n)}">${r.join(``)}</text>`)}function So(e){let t;if(e.startsWith(`canvas:`))t=e.slice(7);else if(e.startsWith(`svg:`))t=e.slice(4);else return{family:`sans-serif`,weight:`400`,style:``};let n=t.indexOf(`@`),r=n>=0?t.slice(0,n):t,i=(n>=0?t.slice(n+1):``).split(`:`),a=i[1],o=i[2],s=r.replace(/[^a-zA-Z0-9 ,\-_.]/g,``)||`sans-serif`,c=`400`;if(a!==void 0){let e=Number(a);Number.isFinite(e)&&e>=1&&e<=1e3&&(c=String(Math.round(e)))}let l=o!==void 0&&new Set([`italic`,`oblique`]).has(o)?o:``;return{family:s,weight:c,style:l}}function Co(e){switch(e){case`center`:return`middle`;case`end`:return`end`;default:return`start`}}function wo(e,t){if(!Number.isFinite(e.font_size))throw RangeError(`PaintVM SVG: font_size must be a finite number, got ${e.font_size}`);let{family:n,weight:r,style:i}=So(e.font_ref),a=[`font-family="${oo(n)}"`,`font-size="${P(e.font_size,`text.font_size`)}"`];r!==`400`&&r!==`normal`&&a.push(`font-weight="${oo(r)}"`),i&&a.push(`font-style="${oo(i)}"`);let o=Co(e.text_align);o!==`start`&&a.push(`text-anchor="${o}"`),t.elements.push(`<text${lo(e.id)} x="${P(e.x,`text.x`)}" y="${P(e.y,`text.y`)}" ${a.join(` `)} fill="${oo(e.fill)}">${so(e.text)}</text>`)}function To(e,t,n){let r=fo(e.transform),i=e.opacity!==void 0&&e.opacity!==1?` opacity="${P(e.opacity,`group.opacity`)}"`:``;t.elements.push(`<g${lo(e.id)}${r}${i}>`);for(let r of e.children)n.dispatch(r,t);t.elements.push(`</g>`)}function Eo(e,t,n){let r=e.id?`filter-${e.id}`:`filter-${t.filterCounter++}`,i=po(r,e.filters);i&&t.defs.push(i);let a=i?` filter="url(#${oo(r)})"`:``,o=e.blend_mode&&e.blend_mode!==`normal`?` style="mix-blend-mode:${ho(e.blend_mode)}"`:``,s=fo(e.transform),c=e.opacity!==void 0&&e.opacity!==1?` opacity="${P(e.opacity,`layer.opacity`)}"`:``;t.elements.push(`<g${lo(e.id)}${s}${c}${a}${o}>`);for(let r of e.children)n.dispatch(r,t);t.elements.push(`</g>`)}function Do(e,t){let n=e.stroke_cap&&F.has(e.stroke_cap)?` stroke-linecap="${e.stroke_cap}"`:``,r=P(e.stroke_width??1,`line.stroke_width`);t.elements.push(`<line${lo(e.id)} x1="${P(e.x1,`line.x1`)}" y1="${P(e.y1,`line.y1`)}" x2="${P(e.x2,`line.x2`)}" y2="${P(e.y2,`line.y2`)}" stroke="${oo(e.stroke)}" stroke-width="${r}"${n} fill="none"/>`)}function Oo(e,t,n){let r=e.id?`clip-${e.id}`:`clip-${t.clipCounter++}`;t.defs.push(`<clipPath id="${oo(r)}"><rect x="${P(e.x,`clip.x`)}" y="${P(e.y,`clip.y`)}" width="${P(e.width,`clip.width`)}" height="${P(e.height,`clip.height`)}"/></clipPath>`),t.elements.push(`<g clip-path="url(#${oo(r)})">`);for(let r of e.children)n.dispatch(r,t);t.elements.push(`</g>`)}function ko(e,t){if(!e.id)return;let n=e.stops.map((e,t)=>`<stop offset="${P(e.offset,`gradient.stops[${t}].offset`)}" stop-color="${oo(e.color)}"/>`).join(``),r;r=e.gradient_kind===`linear`?`<linearGradient id="${oo(e.id)}" x1="${P(e.x1??0,`gradient.x1`)}" y1="${P(e.y1??0,`gradient.y1`)}" x2="${P(e.x2??0,`gradient.x2`)}" y2="${P(e.y2??0,`gradient.y2`)}" gradientUnits="userSpaceOnUse">`+n+`</linearGradient>`:`<radialGradient id="${oo(e.id)}" cx="${P(e.cx??0,`gradient.cx`)}" cy="${P(e.cy??0,`gradient.cy`)}" r="${P(e.r??0,`gradient.r`)}" gradientUnits="userSpaceOnUse">`+n+`</radialGradient>`,t.defs.push(r)}function Ao(e){let t=e.replace(/\0/g,``),n=t.toLowerCase().trimStart();return n.startsWith(`data:`)||n.startsWith(`https:`)?t:`data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==`}function jo(e,t){let n;n=typeof e.src==`string`?Ao(e.src):`data:image/png;base64,`;let r=e.opacity!==void 0&&e.opacity!==1?` opacity="${P(e.opacity,`image.opacity`)}"`:``;t.elements.push(`<image${lo(e.id)} x="${P(e.x,`image.x`)}" y="${P(e.y,`image.y`)}" width="${P(e.width,`image.width`)}" height="${P(e.height,`image.height`)}" href="${oo(n)}"${r}/>`)}function Mo(){let e=new ro((e,t)=>{e.defs.length=0,e.elements.length=0,e.clipCounter=0,e.filterCounter=0},()=>{throw new to(`SVG`)});return e.register(`rect`,(e,t)=>{e.kind===`rect`&&go(e,t)}),e.register(`ellipse`,(e,t)=>{e.kind===`ellipse`&&_o(e,t)}),e.register(`path`,(e,t)=>{e.kind===`path`&&bo(e,t)}),e.register(`glyph_run`,(e,t)=>{e.kind===`glyph_run`&&xo(e,t)}),e.register(`text`,(e,t)=>{e.kind===`text`&&wo(e,t)}),e.register(`group`,(e,t,n)=>{e.kind===`group`&&To(e,t,n)}),e.register(`layer`,(e,t,n)=>{e.kind===`layer`&&Eo(e,t,n)}),e.register(`line`,(e,t)=>{e.kind===`line`&&Do(e,t)}),e.register(`clip`,(e,t,n)=>{e.kind===`clip`&&Oo(e,t,n)}),e.register(`gradient`,(e,t)=>{e.kind===`gradient`&&ko(e,t)}),e.register(`image`,(e,t)=>{e.kind===`image`&&jo(e,t)}),e}function No(e){let t=Mo(),n=ao();return t.execute(e,n),Po(e,n)}function Po(e,t){let n=P(e.width,`scene.width`),r=P(e.height,`scene.height`),i=[];return i.push(`<svg xmlns="http://www.w3.org/2000/svg" width="${n}" height="${r}">`),t.defs.length>0&&i.push(`<defs>${t.defs.join(``)}</defs>`),e.background!==`transparent`&&e.background!==`none`&&i.push(`<rect width="${n}" height="${r}" fill="${oo(e.background)}"/>`),i.push(...t.elements),i.push(`</svg>`),i.join(``)}function Fo(e,t,n,r,i){return{width:e,height:t,background:n,instructions:r,...i}}function Io(e,t,n,r,i){return{kind:`rect`,x:e,y:t,width:n,height:r,...i}}function Lo(e,t,n,r,i){return{kind:`ellipse`,cx:e,cy:t,rx:n,ry:r,...i}}function Ro(e,t,n,r,i,a){return{kind:`line`,x1:e,y1:t,x2:n,y2:r,stroke:i,...a}}function zo(e,t,n,r,i,a,o){return{kind:`text`,x:e,y:t,text:n,font_ref:r,font_size:i,fill:a,...o}}var Bo=`svg:ui-sans-serif@12`,Vo=`#5d6d68`,Ho=`#ffffff`,Uo=`rgba(23, 32, 28, 0.16)`,Wo=`#237a57`,Go=`#2563eb`,Ko=`#c2413b`,qo=`#b7791f`,Jo=`#6d5bd0`,Yo=[`#2563eb`,`#237a57`,`#b7791f`,`#6d5bd0`,`#c2413b`,`#0f766e`,`#be185d`,`#7c3aed`,`#ca8a04`,`#0284c7`];function Xo({model:e,lastStep:t,learningRate:n,lossKind:r,samplePoint:i,pointCount:a}){return(0,E.jsx)(es,{title:`Learning flow`,summary:`Forward pass and gradient descent`,svg:Qo(e,t,n,r,i,a)})}function Zo({example:e,state:t,selectedRow:n,selectedIndex:r,prediction:i,lastStep:a,learningRate:o}){return(0,E.jsx)(es,{title:`Neural graph`,summary:`Hidden layer learning flow`,svg:$o(e,t,n,r,i,a,o)})}function Qo(e,t,n,r=`mse`,i={x:0,y:0},a=1){return No(ts(e,t,n,r,i,a))}function $o(e,t,n,r,i,a,o){return No(ns(e,t,n,r,i,a,o))}function es({title:e,summary:t,svg:n}){return(0,E.jsxs)(`section`,{className:`network-panel`,"aria-label":t,children:[(0,E.jsxs)(`div`,{className:`history__topline`,children:[(0,E.jsx)(`span`,{children:e}),(0,E.jsx)(`strong`,{children:t})]}),(0,E.jsx)(`div`,{className:`network-svg`,dangerouslySetInnerHTML:{__html:n}})]})}function ts(e,t,n,r,i,a){let o=t?.previousState??e,s=i.x*o.weight+o.bias,c=s-i.y,l=r===`mse`?c*c:Math.abs(c),u={id:`input`,label:`x`,value:I(i.x),x:100,y:150,tone:`input`},d={id:`bias`,label:`bias`,value:I(o.bias),x:100,y:232,tone:`bias`},f={id:`sum`,label:`sum`,value:`x*w+b`,x:318,y:150,tone:`hidden`},p={id:`output`,label:`pred`,value:I(s),x:540,y:150,tone:`output`},m={id:`target`,label:`target`,value:I(i.y),x:540,y:232,tone:`bias`},h={id:`loss`,label:r,value:I(l),x:760,y:190,tone:`output`},g=t===null?0:-n*t.gradientWeight,_=t===null?0:-n*t.gradientBias,v=t===null?`waiting for first step`:`dL/dw ${I(t.gradientWeight)}  dL/db ${I(t.gradientBias)}`,y=t===null?`run Step to update weights`:`w ${I(t.previousState.weight)} -> ${I(e.weight)}`,b=t===null?`lr ${I(n)}`:`b ${I(t.previousState.bias)} -> ${I(e.bias)}`;return Fo(920,650,`#ffffff`,[Io(16,16,888,618,{fill:Ho,stroke:Uo,stroke_width:1,corner_radius:8}),...rs(`1 forward pass`,36,48),...ss(u,f,o.weight,`w ${I(o.weight)}`),...ss(d,f,o.bias,`b ${I(o.bias)}`),...ss(f,p,1,`linear`),...ss(p,h,c,`err ${gs(c)}`,.56),...ss(m,h,-1,`truth`,.56),...us(u),...us(d),...us(f),...us(p),...us(m),...us(h),...rs(`2 folded training loop`,36,292),...is(36,322,152,`input batch`,[`${a} rows`,`sample x ${I(i.x)}`,`target ${I(i.y)}`],Go),...as(194,368,242,368,Go,`feed`),...is(252,322,158,`prediction`,[`yhat=x*w+b`,`yhat ${I(s)}`,`activation linear`],Wo),...as(416,368,464,368,Go,`compare`),...is(474,322,162,`error + loss`,[`error ${gs(c)}`,`${r.toUpperCase()} ${I(l)}`,`batch loss ${I(t?.previousLoss??l)}`],Ko),...as(555,448,555,470,Ko,``),...is(474,470,184,`gradient descent`,[v,`dw step ${gs(g)}`,`db step ${gs(_)}`],Jo),...as(464,516,416,516,Jo,`apply lr`),...is(252,470,158,`parameter update`,[y,b,`next run uses them`],Jo),...as(242,516,194,516,Jo,`store`),...is(36,470,152,`model state`,[`w ${I(e.weight)}`,`b ${I(e.bias)}`,`epoch ${e.epoch}`],Go),...ds(`parameter update: new = old - learningRate * gradient`,474,626,13,Vo),...ds(`epoch ${e.epoch}`,36,72,13,Vo),...ds(`line width follows |weight|; green is positive, red is negative`,476,72,12,Vo)],{id:`linear-neural-network-diagram`})}function ns(e,t,n,r,i,a,o){let s=n.input,c=i-n.target,l=wa([s],t.parameters),u=t.parameters.layers.slice(0,-1),d=t.parameters.layers[t.parameters.layers.length-1],f=Math.max(e.inputLabels.length,d.biases.length,...u.map(e=>e.biases.length)),p=Math.max(318,106+Math.max(1,f-1)*66),m=p+92,h=m+344,g=290+Math.max(0,u.length-1)*190+210,_=g+150,v=Math.max(1080,_+98),y=ps(e.inputLabels.length,106,p),b=u.map((e,t)=>290+t*190),x=106+(p-106)*.42,S=x+86,C=x+42,w=e.inputLabels.map((e,t)=>({id:`input-${t}`,label:e,value:I(s[t]??0),x:96,y:y[t],tone:`input`})),ee={id:`bias`,label:`bias`,value:`1`,x:96,y:p+56,tone:`bias`},te=u.map((e,t)=>{let n=ps(e.biases.length,106,p);return e.biases.map((e,r)=>({id:`hidden-${t}-${r}`,label:`h${t+1}.${r+1}`,value:I(l.activationsByLayer[t][0][r]??0),x:b[t],y:n[r],tone:`hidden`}))}),T=te[te.length-1]??[],ne={id:`output`,label:e.outputLabel,value:I(i),x:g,y:x,tone:`output`},re={id:`target`,label:`target`,value:I(n.target),x:g,y:S,tone:`bias`},ie={id:`loss`,label:`mse`,value:I(c*c),x:_,y:C,tone:`output`},E=[Io(16,16,v-32,h-32,{fill:Ho,stroke:Uo,stroke_width:1,corner_radius:8}),...rs(`1 selected forward pass`,32,48),...ds(`epoch ${t.epoch}`,32,70,13,Vo),...ds(`edge color follows source node; line width follows |weight|`,v-412,48,13,Vo)];for(let[e,t]of te.entries()){let n=u[e],r=e===0?w:te[e-1];for(let[i,a]of r.entries())for(let[r,o]of t.entries()){let s=n.weights[i][r],c=e===0&&u.length<=2&&t.length<=8;E.push(...ss(a,o,s,c?I(s):``,.34,cs(a.id)))}}for(let[e,t]of te.entries()){let n=u[e];for(let[r,i]of t.entries()){let a=n.biases[r],o=e===0&&u.length===1&&t.length<=8;E.push(...ss(ee,i,a,o?I(a):``,.26,cs(`bias-${e}`)))}}for(let[e,t]of T.entries()){let n=d.weights[e][0],r=u.length<=2&&T.length<=8;E.push(...ss(t,ne,n,r?I(n):``,.42,cs(t.id)))}E.push(...ss(ee,ne,d.biases[0]??0,u.length===1?I(d.biases[0]??0):``,.28,cs(`bias-output`)),...ss(ne,ie,c,`err ${gs(c)}`,.62),...ss(re,ie,-1,`truth`,.62),...w.flatMap(us),...us(ee),...te.flatMap(e=>e.flatMap(us)),...us(ne),...us(re),...us(ie));let ae=a===null?`input-hidden gradients waiting`:`dL/dW1 ${ms(a.step.weightGradients[0])}`,oe=a?.step.weightGradients[a.step.weightGradients.length-1],se=a?.step.biasGradients[a.step.biasGradients.length-1]?.[0]??0,ce=a?.step.deltas[a.step.deltas.length-1]?.[r]?.[0]??0,D=a?.step.deltas.slice(0,-1).flatMap(e=>e[r]??[])??[],O=D.length===0?0:D.reduce((e,t)=>Math.max(e,Math.abs(t)),0),le=a===null?`waiting for first step`:`max hidden delta ${I(O)}`,ue=m+32,de=m+180;return E.push(...rs(`2 folded loss + update loop`,32,m),...is(32,ue,158,`input row`,[n.label,`inputs ${s.map(e=>I(e)).join(`, `)}`,`target ${I(n.target)}`],Go),...as(196,ue+46,244,ue+46,Go,`forward`),...is(254,ue,162,`prediction`,[`${t.hiddenLayerCount} x hidden[${e.hiddenCount}]`,`${e.outputLabel} ${I(i)}`,`error ${gs(c)}`],Wo),...as(422,ue+46,470,ue+46,Ko,`loss`),...is(480,ue,158,`mse + deltas`,[`row mse ${I(c*c)}`,`output delta ${I(ce)}`,le],Ko),...as(559,ue+126,559,de,Ko,``),...is(480,de,186,`gradient matrices`,[ae,`dL/dW${t.parameters.layers.length} ${ms(oe)}`,`db out ${gs(-o*se)}`],Jo),...as(470,de+46,422,de+46,Jo,`apply lr`),...is(254,de,162,`parameter update`,[`lr ${I(o)}`,`${t.parameters.layers.length} weight matrices`,`next batch uses them`],Jo),...as(244,de+46,196,de+46,Jo,`store`),...is(32,de,158,`model state`,[`epoch ${t.epoch}`,`${t.hiddenLayerCount} hidden layers`,`${e.hiddenCount} neurons/layer`],Go),...ds(`new weights = old weights - learningRate * gradient`,32,h-24,13,Vo),...ds(`scroll the graph to inspect larger networks`,v-336,h-24,12,Vo)),Fo(v,h,`#ffffff`,E,{id:`hidden-neural-network-diagram`})}function rs(e,t,n){return[Io(t-8,n-17,Math.max(126,e.length*8),24,{fill:`rgba(35, 122, 87, 0.1)`,stroke:`rgba(35, 122, 87, 0.18)`,stroke_width:1,corner_radius:6}),...ds(e,t,n,13,Wo)]}function is(e,t,n,r,i,a){let o=[Io(e,t,n,126,{fill:`#ffffff`,stroke:`rgba(23, 32, 28, 0.12)`,stroke_width:1,corner_radius:8}),Io(e,t,n,30,{fill:`rgba(247, 248, 243, 0.95)`,stroke:`rgba(23, 32, 28, 0.08)`,stroke_width:1,corner_radius:8}),...ds(r,e+12,t+21,12,a)];for(let[r,a]of i.entries())o.push(...ds(_s(a,n),e+12,t+54+r*22,11,Vo));return o}function as(e,t,n,r,i,a){return os(e,t,n,r,i,a,2)}function os(e,t,n,r,i,a,o,s=.5,c=-7){let l=Math.atan2(r-t,n-e),u=l+Math.PI*.82,d=l-Math.PI*.82,f=e+(n-e)*s,p=t+(r-t)*s+c,m=[Ro(e,t,n,r,i,{stroke_width:o,stroke_cap:`round`}),Ro(n,r,n+Math.cos(u)*9,r+Math.sin(u)*9,i,{stroke_width:o,stroke_cap:`round`}),Ro(n,r,n+Math.cos(d)*9,r+Math.sin(d)*9,i,{stroke_width:o,stroke_cap:`round`})];return a.length>0&&m.push(...fs(a,f,p,10,i)),m}function ss(e,t,n,r,i=.5,a){let o=e.x+(t.x-e.x)*i,s=e.y+(t.y-e.y)*i,c=a??(n>=0?Wo:Ko),l=Math.min(7,1.4+Math.abs(n)*.75),{x1:u,y1:d,x2:f,y2:p}=ls(e.x,e.y,t.x,t.y,33,36),m=[...os(u,d,f,p,c,``,l)];return r.length>0&&(m.push(Io(o-28,s-14,56,20,{fill:`rgba(255, 255, 255, 0.86)`,stroke:`rgba(23, 32, 28, 0.08)`,stroke_width:1,corner_radius:5})),m.push(...fs(r,o,s+4,10,c))),m}function cs(e){let t=0;for(let n=0;n<e.length;n+=1)t=t*31+e.charCodeAt(n)>>>0;return Yo[t%Yo.length]}function ls(e,t,n,r,i,a){let o=n-e,s=r-t,c=Math.hypot(o,s);if(c===0)return{x1:e,y1:t,x2:n,y2:r};let l=o/c,u=s/c;return{x1:e+l*i,y1:t+u*i,x2:n-l*a,y2:r-u*a}}function us(e){let t=hs(e.tone);return[Lo(e.x,e.y,28,28,{fill:t,stroke:`#ffffff`,stroke_width:3}),Lo(e.x,e.y,31,31,{stroke:Uo,stroke_width:1}),...fs(e.label,e.x,e.y-3,11,`#ffffff`),...fs(e.value,e.x,e.y+12,10,`#ffffff`)]}function ds(e,t,n,r,i,a=`start`){return[zo(t,n,e,Bo,r,i,{text_align:a})]}function fs(e,t,n,r,i){return ds(e,t,n,r,i,`center`)}function ps(e,t,n){if(e<=1)return[(t+n)/2];let r=n-t;return Array.from({length:e},(n,i)=>t+r*i/(e-1))}function ms(e){return e===void 0||e.length===0?`0x0`:`${e.length}x${e[0]?.length??0}`}function hs(e){switch(e){case`input`:return Go;case`hidden`:return Wo;case`output`:return qo;case`bias`:return`#6d5bd0`}}function I(e){return Number.isFinite(e)?Math.abs(e)>=10?e.toFixed(1):e.toFixed(2):`0`}function gs(e){return`${e>=0?`+`:``}${I(e)}`}function _s(e,t){let n=Math.max(10,Math.floor(t/7.2));return e.length<=n?e:`${e.slice(0,n-3)}...`}var L={width:720,height:410,padLeft:58,padRight:24,padTop:24,padBottom:48,xMin:-1,xMax:1,yMin:-.08,yMax:1.08},vs=460;function ys(e,t=3){return Number.isFinite(e)?Math.abs(e)<.01&&e!==0?e.toExponential(2):e.toFixed(t):`0`}function bs(e,t,n){return Math.min(n,Math.max(t,e))}function xs(e){return`${e} hidden layer${e===1?``:`s`}`}function Ss(e,t){let n=t.width-t.padLeft-t.padRight;return t.padLeft+(e-t.xMin)/(t.xMax-t.xMin)*n}function Cs(e,t){let n=t.height-t.padTop-t.padBottom;return t.padTop+(1-(e-t.yMin)/(t.yMax-t.yMin))*n}function ws(e){if(e.length===0)return``;let t=Math.max(...e.map(e=>e.loss),1e-6),n=e[0].epoch,r=Math.max(e[e.length-1].epoch-n,1);return e.map((e,i)=>{let a=(e.epoch-n)/r*250,o=74-bs(e.loss/t,0,1)*74;return`${i===0?`M`:`L`} ${a.toFixed(2)} ${o.toFixed(2)}`}).join(` `)}function Ts(){return Array.from({length:121},(e,t)=>L.xMin+t/120*(L.xMax-L.xMin))}function Es(e,t){return e.map((e,n)=>[e,t[n]??0]).map(([e,t],n)=>`${n===0?`M`:`L`} ${Ss(e,L).toFixed(2)} ${Cs(t,L).toFixed(2)}`).join(` `)}function Ds(e,t){let n=Ts();return Es(n,Aa(n.map(e=>[e]),t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}).predictions.map(e=>e[0]??0))}function Os(e){let t=[],n=[],r=e.rows.map(e=>e.input[0]),i=e.rows.map(e=>e.input[1]),a=Math.min(...r,-1),o=Math.max(...r,1),s=Math.min(...i,-1),c=Math.max(...i,1),l=Math.max((o-a)*.08,.15),u=Math.max((c-s)*.08,.15);for(let e=0;e<26;e+=1)for(let r=0;r<26;r+=1){let i=a-l+r/25*(o-a+l*2),d=s-u+e/25*(c-s+u*2);n.push([i,d]),t.push({x:r,y:e,value:0})}return{cells:t,inputs:n}}function ks(e,t){let n=Os(e),r=Aa(n.inputs,t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}).predictions;return n.cells.map((e,t)=>({...e,value:r[t]?.[0]??0}))}function As(e,t){return{rowPredictions:Ua(e,t),curvePath:e.chartKind===`curve`?Ds(e,t):``,surfaceCells:e.chartKind===`surface`?ks(e,t):[],backend:`cpu`}}async function js(e,t){let n=Va(e),r=e.chartKind===`curve`?Ts():[],i=r.map(e=>[e]),a=e.chartKind===`surface`?Os(e):{cells:[],inputs:[]},o=await ja([...n,...i,...a.inputs],t.parameters,{inputNames:e.inputLabels,outputNames:[e.outputLabel]}),s=o.predictions.map(e=>e[0]??0),c=s.slice(0,n.length),l=s.slice(n.length,n.length+i.length),u=s.slice(n.length+i.length);return{rowPredictions:c,curvePath:r.length>0?Es(r,l):``,surfaceCells:a.cells.map((e,t)=>({...e,value:u[t]??0})),backend:o.backend,fallbackReason:o.fallbackReason}}function Ms({example:e,curvePath:t,predictions:n}){return(0,E.jsxs)(`svg`,{viewBox:`0 0 ${L.width} ${L.height}`,role:`img`,"aria-label":`${e.title} hidden-layer curve`,children:[(0,E.jsx)(`rect`,{className:`plot-bg`,x:L.padLeft,y:L.padTop,width:L.width-L.padLeft-L.padRight,height:L.height-L.padTop-L.padBottom}),[0,.25,.5,.75,1].map(e=>{let t=L.xMin+(L.xMax-L.xMin)*e,n=L.yMin+(L.yMax-L.yMin)*e;return(0,E.jsxs)(`g`,{children:[(0,E.jsx)(`line`,{className:`grid-line`,x1:Ss(t,L),x2:Ss(t,L),y1:L.padTop,y2:L.height-L.padBottom}),(0,E.jsx)(`text`,{className:`axis-label`,x:Ss(t,L),y:L.height-18,children:ys(t,1)}),(0,E.jsx)(`line`,{className:`grid-line`,x1:L.padLeft,x2:L.width-L.padRight,y1:Cs(n,L),y2:Cs(n,L)}),(0,E.jsx)(`text`,{className:`axis-label axis-label--y`,x:L.padLeft-10,y:Cs(n,L)+4,children:ys(n,1)})]},e)}),(0,E.jsx)(`path`,{className:`hidden-curve`,d:t}),e.rows.map((e,t)=>{let r=Ss(e.input[0],L),i=Cs(e.target,L),a=Cs(n[t],L);return(0,E.jsxs)(`g`,{children:[(0,E.jsx)(`line`,{className:`error-line`,x1:r,x2:r,y1:i,y2:a}),(0,E.jsx)(`circle`,{className:`truth-point`,cx:r,cy:i,r:`6`}),(0,E.jsx)(`circle`,{className:`prediction-point`,cx:r,cy:a,r:`5`})]},e.label)}),(0,E.jsx)(`text`,{className:`axis-title`,x:L.width/2,y:L.height-5,children:e.inputLabels[0]}),(0,E.jsx)(`text`,{className:`axis-title axis-title--y`,x:`20`,y:L.height/2,children:e.outputLabel})]})}function Ns({example:e,cells:t,predictions:n,selectedIndex:r,onSelect:i}){let a=vs/Math.sqrt(t.length),o=e.rows.map(e=>e.input[0]),s=e.rows.map(e=>e.input[1]),c=Math.min(...o,-1),l=Math.max(...o,1),u=Math.min(...s,-1),d=Math.max(...s,1),f=Math.max((l-c)*.08,.15),p=Math.max((d-u)*.08,.15),m=e=>(e-(c-f))/(l-c+f*2)*vs,h=e=>vs-(e-(u-p))/(d-u+p*2)*vs;return(0,E.jsxs)(`svg`,{className:`surface-chart`,viewBox:`0 0 ${vs} ${vs}`,role:`img`,"aria-label":`${e.title} decision surface`,children:[t.map(e=>(0,E.jsx)(`rect`,{x:e.x*a,y:e.y*a,width:a+.5,height:a+.5,style:{fill:`rgba(${Math.round(194-e.value*150)}, ${Math.round(65+e.value*90)}, ${Math.round(59+e.value*120)}, 0.72)`}},`${e.x}-${e.y}`)),e.rows.map((e,t)=>(0,E.jsxs)(`g`,{"aria-label":`Select ${e.label}`,className:`svg-button`,role:`button`,tabIndex:0,onClick:()=>i(t),onKeyDown:e=>{(e.key===`Enter`||e.key===` `)&&i(t)},children:[(0,E.jsx)(`circle`,{className:t===r?`surface-point surface-point--selected`:`surface-point`,cx:m(e.input[0]),cy:h(e.input[1]),r:t===r?9:7,style:{fill:e.target>=.5?`#237a57`:`#f7f8f3`}}),(0,E.jsx)(`text`,{className:`surface-label`,x:m(e.input[0])+10,y:h(e.input[1])-8,children:ys(n[t],2)})]},e.label))]})}function Ps({example:e,predictions:t,selectedIndex:n,onSelect:r}){return(0,E.jsx)(`div`,{className:`hidden-table-chart`,children:e.rows.map((e,i)=>{let a=t[i]-e.target;return(0,E.jsxs)(`button`,{className:i===n?`table-row table-row--selected`:`table-row`,type:`button`,onClick:()=>r(i),children:[(0,E.jsx)(`span`,{children:e.label}),(0,E.jsxs)(`span`,{className:`bar-pair`,children:[(0,E.jsx)(`i`,{className:`bar-target`,style:{width:`${e.target*100}%`}}),(0,E.jsx)(`i`,{className:`bar-prediction`,style:{width:`${t[i]*100}%`}})]}),(0,E.jsx)(`code`,{children:ys(a,3)})]},e.label)})})}function Fs(){let[e,t]=(0,l.useState)(Qa[0].id),n=Qa.find(t=>t.id===e)??Qa[0],[r,i]=(0,l.useState)(n.defaultLearningRate),[a,o]=(0,l.useState)(()=>Ha(n)),[s,c]=(0,l.useState)(()=>[Ka(n,Ha(n))]),[u,d]=(0,l.useState)(null),[f,p]=(0,l.useState)(0),[m,h]=(0,l.useState)(!1),g=a.hiddenLayerCount;(0,l.useEffect)(()=>{let e=Ha(n);i(n.defaultLearningRate),o(e),c([Ka(n,e)]),d(null),p(0),h(!1)},[n]);let _=(0,l.useMemo)(()=>As(n,a),[n,a]),[v,y]=(0,l.useState)(null),b=v??_,x=b.rowPredictions,S=(0,l.useMemo)(()=>Wa(n,a),[n,a]),C=(0,l.useMemo)(()=>Ga(n,a),[n,a]),w=(0,l.useMemo)(()=>Ya(n,a,f),[n,f,a]),ee=u?.step.weightGradients[u.step.weightGradients.length-1],te=b.backend===`webgpu`?`WebGPU`:`CPU`;(0,l.useEffect)(()=>{let e=!1;return y(null),Ma()&&js(n,a).then(t=>{e||y(t)}).catch(t=>{e||y({..._,fallbackReason:t instanceof Error?t.message:`Matrix backend failed`})}),()=>{e=!0}},[n,_,a]);function T(e){o(e.state),d(e),c(t=>[...t.slice(-159),{epoch:e.state.epoch,loss:e.loss,mae:e.mae}])}function ne(e){let t=Ja(n,a,r,e),i=t[t.length-1];i!==void 0&&T(i)}function re(){let e=Ha(n,g);o(e),c([Ka(n,e)]),d(null),h(!1)}function ie(e){let n=Ha(e);t(e.id),i(e.defaultLearningRate),o(n),c([Ka(e,n)]),d(null),p(0),h(!1)}function ae(e){let t=Ha(n,Math.max(n.hiddenLayerMin,Math.min(n.hiddenLayerMax,Math.round(e))));o(t),c([Ka(n,t)]),d(null),p(0),h(!1)}return(0,l.useEffect)(()=>{if(!m)return;let e=window.setInterval(()=>{o(e=>{let t=Ja(n,e,r,5),i=t[t.length-1];return d(i),c(e=>[...e.slice(-159),{epoch:i.state.epoch,loss:i.loss,mae:i.mae}]),i.state})},160);return()=>window.clearInterval(e)},[n,m,r]),(0,E.jsxs)(`main`,{className:`workspace workspace--hidden`,children:[(0,E.jsxs)(`nav`,{className:`lab-rail`,"aria-label":`Hidden-layer examples`,children:[(0,E.jsxs)(`div`,{className:`rail-summary`,children:[(0,E.jsx)(`strong`,{children:Qa.length}),(0,E.jsx)(`span`,{children:`hidden examples`})]}),(0,E.jsx)(`div`,{className:`lab-list`,children:Qa.map(e=>(0,E.jsxs)(`button`,{className:e.id===n.id?`lab-button lab-button--active`:`lab-button`,type:`button`,onClick:()=>ie(e),children:[(0,E.jsx)(`span`,{children:e.title}),(0,E.jsx)(`small`,{children:e.category})]},e.id))})]}),(0,E.jsxs)(`section`,{className:`lab-stage`,"aria-label":`Hidden-layer training stage`,children:[(0,E.jsxs)(`div`,{className:`lab-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:n.category}),(0,E.jsx)(`h2`,{children:n.title}),(0,E.jsx)(`p`,{children:n.summary})]}),(0,E.jsxs)(`div`,{className:`lab-chip`,children:[g,` layers / `,n.hiddenCount,` neurons`]})]}),(0,E.jsxs)(`section`,{className:`chart-panel chart-panel--hidden`,"aria-label":`Hidden-layer chart`,children:[n.chartKind===`curve`&&(0,E.jsx)(Ms,{example:n,curvePath:b.curvePath,predictions:x}),n.chartKind===`surface`&&(0,E.jsx)(Ns,{example:n,cells:b.surfaceCells,predictions:x,selectedIndex:f,onSelect:p}),n.chartKind===`table`&&(0,E.jsx)(Ps,{example:n,predictions:x,selectedIndex:f,onSelect:p}),(0,E.jsxs)(`div`,{className:`legend`,"aria-label":`Hidden chart legend`,children:[(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-dot legend-dot--truth`}),`Target`]}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-dot legend-dot--prediction`}),`Prediction`]}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-line legend-line--model`}),`Current model`]})]})]}),(0,E.jsxs)(`section`,{className:`trace-panel`,"aria-label":`Neuron trace`,children:[(0,E.jsxs)(`div`,{className:`history__topline`,children:[(0,E.jsx)(`span`,{children:n.rows[f].label}),(0,E.jsxs)(`strong`,{children:[ys(x[f],3),` / `,ys(n.rows[f].target,3)]})]}),(0,E.jsx)(`div`,{className:`hidden-neuron-grid`,children:w.layers.filter(e=>e.layer.startsWith(`hidden`)).flatMap((e,t)=>e.neurons.map((e,n)=>(0,E.jsxs)(`div`,{className:`neuron-tile`,children:[(0,E.jsxs)(`span`,{children:[`h`,t+1,`.`,n+1]}),(0,E.jsx)(`strong`,{children:ys(e.output,3)}),(0,E.jsx)(`i`,{style:{width:`${bs(e.output,0,1)*100}%`}})]},e.neuron)))}),(0,E.jsx)(`div`,{className:`trace-equation`,children:(0,E.jsxs)(`code`,{children:[n.inputLabels.join(`, `),` `,`->`,` `,g,` x hidden[`,n.hiddenCount,`] `,`->`,` `,n.outputLabel]})})]}),(0,E.jsx)(Zo,{example:n,state:a,selectedRow:n.rows[f],selectedIndex:f,prediction:x[f],lastStep:u,learningRate:r})]}),(0,E.jsxs)(`aside`,{className:`controls metrics`,"aria-label":`Hidden-layer controls`,children:[(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Hidden layers`}),(0,E.jsx)(`input`,{type:`range`,min:n.hiddenLayerMin,max:n.hiddenLayerMax,step:`1`,value:g,onChange:e=>ae(Number(e.target.value))}),(0,E.jsx)(`input`,{type:`number`,min:n.hiddenLayerMin,max:n.hiddenLayerMax,step:`1`,value:g,onChange:e=>ae(Number(e.target.value))})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Learning rate`}),(0,E.jsx)(`input`,{type:`range`,min:n.learningRateMin,max:n.learningRateMax,step:n.learningRateStep,value:r,onChange:e=>i(Number(e.target.value))}),(0,E.jsx)(`input`,{type:`number`,min:n.learningRateMin,max:n.learningRateMax,step:n.learningRateStep,value:r,onChange:e=>i(Number(e.target.value))})]}),(0,E.jsxs)(`div`,{className:`button-grid`,children:[(0,E.jsx)(`button`,{type:`button`,onClick:()=>ne(1),children:`Step`}),(0,E.jsx)(`button`,{type:`button`,onClick:()=>ne(25),children:`Step 25`}),(0,E.jsx)(`button`,{type:`button`,onClick:()=>h(e=>!e),children:m?`Pause`:`Run`}),(0,E.jsx)(`button`,{type:`button`,onClick:re,children:`Reset`})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Epoch`}),(0,E.jsx)(`strong`,{children:a.epoch})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Loss`}),(0,E.jsx)(`strong`,{children:ys(S,4)})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Average error`}),(0,E.jsx)(`strong`,{children:ys(C,3)})]}),(0,E.jsxs)(`div`,{className:`metric`,title:b.fallbackReason,children:[(0,E.jsx)(`span`,{children:`Matrix backend`}),(0,E.jsx)(`strong`,{children:te})]}),(0,E.jsxs)(`div`,{className:`history`,children:[(0,E.jsxs)(`div`,{className:`history__topline`,children:[(0,E.jsx)(`span`,{children:`Loss history`}),(0,E.jsxs)(`strong`,{children:[s.length,` points`]})]}),(0,E.jsxs)(`svg`,{viewBox:`0 0 250 74`,role:`img`,"aria-label":`Hidden-layer loss history`,children:[(0,E.jsx)(`path`,{className:`history-grid`,d:`M 0 37 L 250 37`}),(0,E.jsx)(`path`,{className:`history-line`,d:ws(s)})]})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Trace row`}),(0,E.jsx)(`select`,{value:f,onChange:e=>p(Number(e.target.value)),children:n.rows.map((e,t)=>(0,E.jsx)(`option`,{value:t,children:e.label},e.label))})]}),(0,E.jsxs)(`div`,{className:`gradients`,children:[(0,E.jsx)(`span`,{children:`Last gradient shape`}),(0,E.jsx)(`code`,{children:xs(g)}),(0,E.jsxs)(`code`,{children:[`input-hidden `,Oa(u?.step.weightGradients[0])]}),(0,E.jsxs)(`code`,{children:[`hidden-output `,Oa(ee)]})]}),(0,E.jsxs)(`div`,{className:`lesson`,children:[(0,E.jsx)(`span`,{children:`Learning note`}),(0,E.jsx)(`p`,{children:n.lesson})]})]})]})}var Is=[{name:`vertical-position`,values:[[0,0,0],[1,1,1],[2,2,2]]},{name:`horizontal-position`,values:[[0,1,2],[0,1,2],[0,1,2]]}],Ls=[{name:`toward-bottom-right`,kernels:[[[4,0],[0,0]],[[2,0],[0,0]]],bias:0},{name:`toward-top-left`,kernels:[[[-4,0],[0,0]],[[-2,0],[0,0]]],bias:6}],Rs=[1,1],zs=[0,0];function Bs(e){return e===0?0:e}function Vs(e){if(e.length===0||e[0].length===0)throw Error(`Matrices must contain at least one value.`);let t=e[0].length;if(e.some(e=>e.length!==t||!e.every(Number.isFinite)))throw Error(`Matrices must be rectangular and contain finite numbers.`);return[e.length,t]}function Hs(e,t,n,r){if(!Number.isFinite(t)||t<=0)throw Error(`Normalization epsilon must be positive.`);if(n.length!==e.length||r.length!==e.length)throw Error(`Gamma and beta must match the output channel count.`);let i=[],a=[],o=[];return{means:i,variances:a,denominators:o,maps:e.map((e,s)=>{let c=e.flat(),l=c.reduce((e,t)=>e+t,0)/c.length,u=c.reduce((e,t)=>e+(t-l)**2,0)/c.length,d=Math.sqrt(u+t);return i.push(l),a.push(u),o.push(d),e.map(e=>e.map(e=>Bs(n[s]*(e-l)/d+r[s])))})}}function Us(e){let t=[],n=[];for(let r of e){let e=-1/0,i=[0,0];for(let[t,n]of r.entries())for(let[r,a]of n.entries())a>e&&(e=a,i=[t,r]);t.push(e),n.push(i)}return{values:t,argmax:n}}function Ws(e=Is,t=Ls,n=4,r=Rs,i=zs){if(e.length===0||t.length===0)throw Error(`The image and filter bank must be non-empty.`);let[a,o]=Vs(e[0].values);if(e.some(e=>{let t=Vs(e.values);return t[0]!==a||t[1]!==o}))throw Error(`Every input channel must have the same image shape.`);let s=[],c=[],l=[];for(let[n,r]of t.entries()){if(!Number.isFinite(r.bias)||r.kernels.length!==e.length)throw Error(`Every filter needs a finite bias and one kernel per input channel.`);let[t,i]=Vs(r.kernels[0]);if(r.kernels.some(e=>{let n=Vs(e);return n[0]!==t||n[1]!==i}))throw Error(`Every kernel in one filter must have the same shape.`);if(t>a||i>o)throw Error(`Kernels must fit inside the image in valid mode.`);let u=a-t+1,d=o-i+1,f=e.map(()=>Array.from({length:u},()=>Array(d).fill(0))),p=[],m=[];for(let a=0;a<u;a+=1){let o=[],s=[];for(let c=0;c<d;c+=1){let l=e.map(e=>Array.from({length:t},(t,n)=>e.values[a+n].slice(c,c+i))),u=l.map((e,t)=>e.map((e,n)=>e.map((e,i)=>Bs(e*r.kernels[t][n][i])))),d=u.map(e=>Bs(e.flat().reduce((e,t)=>e+t,0))),p=Bs(d.reduce((e,t)=>e+t,0)),m=Bs(p+r.bias);d.forEach((e,t)=>{f[t][a][c]=e}),o.push({filterIndex:n,row:a,column:c,windows:l,products:u,channelSums:d,preBiasSum:p,output:m}),s.push(m)}p.push(o),m.push(s)}s.push(p),c.push(f),l.push(m)}let u=Hs(l,n,r,i),d=u.maps.map(e=>e.map(e=>e.map(e=>Math.max(0,e))));return{positions:s,channelContributions:c,convolution:l,normalization:u,activation:d,pooling:Us(d)}}var Gs=[{id:`channels`,label:`Channels`},{id:`convolve`,label:`Convolve`},{id:`normalize`,label:`Normalize`},{id:`relu`,label:`ReLU`},{id:`pool`,label:`Pool`}];function Ks(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function qs({values:e,label:t,selected:n,winner:r}){return(0,E.jsxs)(`div`,{className:`image-matrix-block`,children:[(0,E.jsx)(`span`,{children:t}),(0,E.jsx)(`div`,{className:`image-matrix`,style:{gridTemplateColumns:`repeat(${e[0].length}, minmax(44px, 1fr))`},"aria-label":t,children:e.flatMap((e,t)=>e.map((e,i)=>{let a=n?.[0]===t&&n[1]===i;return(0,E.jsxs)(`div`,{className:r?.[0]===t&&r[1]===i?`image-matrix-cell image-matrix-cell--winner`:a?`image-matrix-cell image-matrix-cell--selected`:`image-matrix-cell`,children:[(0,E.jsxs)(`small`,{children:[`[`,t,`,`,i,`]`]}),(0,E.jsx)(`strong`,{children:Ks(e)})]},`${t}-${i}`)}))})]})}function Js(){let[e,t]=(0,l.useState)(`channels`),[n,r]=(0,l.useState)(0),[i,a]=(0,l.useState)(3),o=(0,l.useMemo)(()=>Ws(),[]),s=Gs.findIndex(t=>t.id===e),c=Math.floor(i/2),u=i%2,d=Ls[n],f=o.positions[n][c][u],p=[c,u];function m(e){t(Gs[Math.min(Math.max(s+e,0),Gs.length-1)].id)}function h(){t(`channels`),r(0),a(3)}return(0,E.jsxs)(`main`,{className:`workspace workspace--image-cnn`,children:[(0,E.jsxs)(`section`,{className:`image-cnn-stage`,"aria-label":`Tiny image CNN trace`,children:[(0,E.jsxs)(`div`,{className:`image-cnn-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN07 · tiny image CNN`}),(0,E.jsx)(`h2`,{children:`Open the image pipeline`}),(0,E.jsx)(`p`,{children:`Follow two image channels through shared spatial windows, channel reduction, normalization, ReLU, and max pooling.`})]}),(0,E.jsx)(`div`,{className:`image-shape-chip`,children:`2 × 3 × 3 → 2 × 2 × 2 → 2`})]}),(0,E.jsx)(`nav`,{className:`image-pipeline`,"aria-label":`Image CNN pipeline stages`,children:Gs.map((n,r)=>(0,E.jsxs)(`button`,{"aria-label":`Show ${n.label} stage`,className:n.id===e?`image-stage-button image-stage-button--active`:r<s?`image-stage-button image-stage-button--visited`:`image-stage-button`,type:`button`,onClick:()=>t(n.id),children:[(0,E.jsx)(`small`,{children:r+1}),(0,E.jsx)(`strong`,{children:n.label})]},n.id))}),e===`channels`?(0,E.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Input image channels`,children:[(0,E.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Stage 1 · input tensor`}),(0,E.jsx)(`h2`,{children:`One image can have several number grids`})]}),(0,E.jsx)(`code`,{children:`shape [channels, rows, columns] = [2, 3, 3]`})]}),(0,E.jsx)(`div`,{className:`image-channel-grid`,children:Is.map((e,t)=>(0,E.jsxs)(`article`,{className:`image-channel-card`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`input channel `,t]}),(0,E.jsx)(`strong`,{children:e.name})]}),(0,E.jsx)(qs,{values:e.values,label:`${e.name} values`})]},e.name))}),(0,E.jsx)(`p`,{className:`image-stage-note`,children:`A filter owns one kernel per input channel. Their spatial results meet only after each channel has produced its own partial sum.`})]}):null,e===`convolve`?(0,E.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Channel accumulation trace`,children:[(0,E.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Stage 2 · filter `,n,` · output [`,c,`,`,u,`]`]}),(0,E.jsx)(`h2`,{children:`Correlate each channel, then add`})]}),(0,E.jsx)(`strong`,{className:`image-output-value`,children:Ks(f.output)})]}),(0,E.jsx)(`div`,{className:`channel-math-grid`,children:Is.map((e,t)=>(0,E.jsxs)(`article`,{className:`channel-math-card`,children:[(0,E.jsxs)(`div`,{className:`channel-math-title`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`channel `,t]}),(0,E.jsx)(`strong`,{children:e.name})]}),(0,E.jsx)(`strong`,{children:Ks(f.channelSums[t])})]}),(0,E.jsxs)(`div`,{className:`window-kernel-pair`,children:[(0,E.jsx)(qs,{values:f.windows[t],label:`selected window`}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`×`}),(0,E.jsx)(qs,{values:d.kernels[t],label:`channel kernel`})]}),(0,E.jsx)(`div`,{className:`image-product-list`,children:f.products[t].flatMap((e,n)=>e.map((e,r)=>(0,E.jsxs)(`code`,{children:[Ks(f.windows[t][n][r]),`×`,Ks(d.kernels[t][n][r]),`=`,Ks(e)]},`${n}-${r}`)))})]},e.name))}),(0,E.jsxs)(`div`,{className:`channel-reduction`,"aria-label":`Channel reduction equation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`channel 0`}),(0,E.jsx)(`strong`,{children:Ks(f.channelSums[0])})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`channel 1`}),(0,E.jsx)(`strong`,{children:Ks(f.channelSums[1])})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias`}),(0,E.jsx)(`strong`,{children:Ks(d.bias)})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{className:`channel-reduction__result`,children:[(0,E.jsx)(`small`,{children:`output`}),(0,E.jsx)(`strong`,{children:Ks(f.output)})]})]}),(0,E.jsx)(`div`,{className:`image-map-pair`,children:o.convolution.map((e,t)=>(0,E.jsx)(qs,{values:e,label:`filter ${t} convolution map`,selected:t===n?p:void 0},t))})]}):null,e===`normalize`?(0,E.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Spatial normalization trace`,children:[(0,E.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Stage 3 · output channel `,n]}),(0,E.jsx)(`h2`,{children:`Four spatial values share statistics`})]}),(0,E.jsx)(`code`,{children:`(x − μ) / √(variance + ε)`})]}),(0,E.jsxs)(`div`,{className:`normalization-flow`,children:[(0,E.jsx)(qs,{values:o.convolution[n],label:`convolution map`,selected:p}),(0,E.jsxs)(`div`,{className:`normalization-stats`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean μ`}),(0,E.jsx)(`strong`,{children:Ks(o.normalization.means[n])})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`variance`}),(0,E.jsx)(`strong`,{children:Ks(o.normalization.variances[n])})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`epsilon ε`}),(0,E.jsx)(`strong`,{children:Ks(4)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`denominator`}),(0,E.jsx)(`strong`,{children:Ks(o.normalization.denominators[n])})]})]}),(0,E.jsx)(qs,{values:o.normalization.maps[n],label:`normalized map`,selected:p})]}),(0,E.jsxs)(`code`,{className:`normalization-equation`,children:[`(`,Ks(f.output),` − `,Ks(o.normalization.means[n]),`)`,` `,`/ `,Ks(o.normalization.denominators[n]),` `,`× γ `,Ks(Rs[n]),` `,`+ β `,Ks(zs[n]),` `,`= `,Ks(o.normalization.maps[n][c][u])]})]}):null,e===`relu`?(0,E.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`ReLU activation trace`,children:[(0,E.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Stage 4 · output channel `,n]}),(0,E.jsx)(`h2`,{children:`Keep positive evidence`})]}),(0,E.jsx)(`code`,{children:`ReLU(x) = max(0, x)`})]}),(0,E.jsxs)(`div`,{className:`activation-flow`,children:[(0,E.jsx)(qs,{values:o.normalization.maps[n],label:`normalized values`,selected:p}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,E.jsx)(qs,{values:o.activation[n],label:`after ReLU`,selected:p})]}),(0,E.jsxs)(`code`,{className:`normalization-equation`,children:[`max(0, `,Ks(o.normalization.maps[n][c][u]),`)`,` `,`= `,Ks(o.activation[n][c][u])]})]}):null,e===`pool`?(0,E.jsxs)(`section`,{className:`image-stage-panel`,"aria-label":`Max pooling trace`,children:[(0,E.jsxs)(`div`,{className:`image-stage-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Stage 5 · shrink the maps`}),(0,E.jsx)(`h2`,{children:`Keep each channel's strongest location`})]}),(0,E.jsx)(`code`,{children:`2 × 2 max pool · stride 2`})]}),(0,E.jsx)(`div`,{className:`pooling-grid`,children:o.activation.map((e,t)=>(0,E.jsxs)(`article`,{className:`pooling-card`,children:[(0,E.jsx)(qs,{values:e,label:`filter ${t} activated map`,winner:o.pooling.argmax[t]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,E.jsxs)(`div`,{className:`pooled-value`,children:[(0,E.jsxs)(`small`,{children:[`pooled[`,t,`]`]}),(0,E.jsx)(`strong`,{children:Ks(o.pooling.values[t])}),(0,E.jsxs)(`code`,{children:[`from [`,o.pooling.argmax[t][0],`,`,o.pooling.argmax[t][1],`]`]})]})]},t))}),(0,E.jsx)(`p`,{className:`image-stage-note`,children:`Only the highlighted winner receives gradient through max pooling. The other three values were useful for comparison, but are discarded.`})]}):null]}),(0,E.jsxs)(`aside`,{className:`image-cnn-controls`,"aria-label":`Image CNN trace controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Choose one path`}),(0,E.jsx)(`h2`,{children:`Filter and output`}),(0,E.jsx)(`p`,{children:`Selections stay synchronized as you move through the pipeline.`}),(0,E.jsxs)(`div`,{className:`image-control-group`,children:[(0,E.jsx)(`span`,{children:`Output filter`}),(0,E.jsx)(`div`,{className:`image-filter-buttons`,children:Ls.map((e,t)=>(0,E.jsxs)(`button`,{"aria-label":`Select filter ${t} ${e.name}`,className:t===n?`image-choice image-choice--active`:`image-choice`,type:`button`,onClick:()=>r(t),children:[(0,E.jsxs)(`small`,{children:[`filter `,t]}),(0,E.jsx)(`strong`,{children:e.name})]},e.name))})]}),(0,E.jsxs)(`div`,{className:`image-control-group`,children:[(0,E.jsx)(`span`,{children:`Spatial output`}),(0,E.jsx)(`div`,{className:`image-position-buttons`,children:[0,1,2,3].map(e=>{let t=Math.floor(e/2),r=e%2;return(0,E.jsxs)(`button`,{"aria-label":`Select image output row ${t} column ${r}`,className:e===i?`image-choice image-choice--active`:`image-choice`,type:`button`,onClick:()=>a(e),children:[(0,E.jsxs)(`small`,{children:[`[`,t,`,`,r,`]`]}),(0,E.jsx)(`strong`,{children:Ks(o.convolution[n][t][r])})]},e)})})]}),(0,E.jsxs)(`div`,{className:`button-grid image-stage-controls`,children:[(0,E.jsx)(`button`,{type:`button`,disabled:s===0,onClick:()=>m(-1),children:`Previous stage`}),(0,E.jsx)(`button`,{type:`button`,disabled:s===Gs.length-1,onClick:()=>m(1),children:`Next stage`}),(0,E.jsx)(`button`,{type:`button`,onClick:h,children:`Reset trace`})]}),(0,E.jsxs)(`div`,{className:`image-cnn-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Larger CNNs repeat these same loops over batches, many channels, many filters, and deeper feature maps. Accelerators change the schedule, not the arithmetic contract.`})]})]})]})}var Ys=`species,island,bill_length_mm,bill_depth_mm,flipper_length_mm,body_mass_g,sex,year
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
`,Xs=[`species`,`island`,`bill_length_mm`,`bill_depth_mm`,`flipper_length_mm`,`body_mass_g`,`sex`,`year`];function Zs(e){return Number(e)}function Qs(e){return e.trim().split(`
`).slice(1).map(e=>{let t=e.split(`,`),n=Object.fromEntries(Xs.map((e,n)=>[e,t[n]??``]));return{species:n.species,island:n.island,bill_length_mm:Zs(n.bill_length_mm),bill_depth_mm:Zs(n.bill_depth_mm),flipper_length_mm:Zs(n.flipper_length_mm),body_mass_g:Zs(n.body_mass_g),sex:n.sex,year:Zs(n.year)}}).filter(e=>Number.isFinite(e.bill_length_mm)&&Number.isFinite(e.body_mass_g))}var $s=Qs(Ys);function ec(e,t){return $s.map(n=>({x:n[e],y:n[t],label:`${n.species} on ${n.island}`,group:n.species}))}function tc(e,t){return ka(e.map(e=>e.x),t).predictions}function nc(e,t,n){let r=tc(e,t);return n===`mse`?e.reduce((e,t,n)=>{let i=r[n]-t.y;return e+i*i},0)/e.length:e.reduce((e,t,n)=>e+Math.abs(r[n]-t.y),0)/e.length}function rc(e,t){return nc(e,t,`mae`)}function ic(e,t,n){let r=tc(e,t),i=e.length;return e.reduce((e,t,a)=>{let o=r[a]-t.y,s=n===`mse`?2/i*o:Math.sign(o)/i;return{gradientWeight:e.gradientWeight+s*t.x,gradientBias:e.gradientBias+s}},{gradientWeight:0,gradientBias:0})}function ac(e,t,n,r){let{gradientWeight:i,gradientBias:a}=ic(e,t,r),o=nc(e,t,r),s={weight:t.weight-n*i,bias:t.bias-n*a,epoch:t.epoch+1};return{previousState:t,previousLoss:o,state:s,loss:nc(e,s,r),mae:rc(e,s),gradientWeight:i,gradientBias:a}}function oc(e,t,n,r,i){let a=[],o=t;for(let t=0;t<i;t+=1){let t=ac(e,o,n,r);a.push(t),o=t.state}return a}function sc(e){let t=e.length,n=e.reduce((e,t)=>e+t.x,0)/t,r=e.reduce((e,t)=>e+t.y,0)/t,i=e.reduce((e,t)=>e+(t.x-n)*(t.y-r),0),a=e.reduce((e,t)=>e+(t.x-n)**2,0),o=a===0?0:i/a;return{weight:o,bias:r-o*n,epoch:0}}var cc={name:`Generated in browser from deterministic formulas`,kind:`synthetic`,license:`Generated example data`},lc={name:`Palmer Penguins sample`,kind:`local-csv`,license:`CC0 1.0 Universal`,url:`https://github.com/allisonhorst/palmerpenguins`},uc=[-8,-6,-4,-2,0,2,4,6,8],dc=[-40,-10,0,8,15,22,38,60,100],fc=[0,.12,.25,.38,.5,.62,.75,.88,1];function pc(e){return e.toLowerCase().replace(/[^a-z0-9]+/g,`-`).replace(/^-|-$/g,``)}function mc(e,t,n={}){let r=n.xs??uc,i=n.noise??0,a=n.seed??1,o=n.curve??0;return r.map((r,s)=>{let c=Math.sin((s+1)*(a+1.7))*i,l=s===n.outlierIndex?n.outlierShift??0:0;return{x:r,y:e*r+t+o*r*r+c+l}})}function hc(e){let t=sc(e.points),n=e.defaultLearningRate??.01;return{id:pc(`${e.category}-${e.title}`),title:e.title,category:e.category,summary:e.summary,lesson:e.lesson,xLabel:e.xLabel??`Input`,yLabel:e.yLabel??`Target`,points:e.points,defaultLoss:e.defaultLoss??`mse`,defaultLearningRate:n,learningRateMin:n/20,learningRateMax:n*40,learningRateStep:n/20,initialModel:e.initialModel??{weight:0,bias:0,epoch:0},idealModel:t,source:e.source??cc}}var gc=[[`Celsius to Fahrenheit`,`Exact unit conversion with a real slope and intercept.`,1.8,32,dc,5e-4],[`Inches to centimeters`,`A clean proportional relationship with almost no intercept.`,2.54,0,uc,.01],[`Miles to kilometers`,`Another unit conversion where the slope carries the lesson.`,1.609,0,uc,.01],[`Hours to wages`,`A wage model where the intercept acts like a fixed bonus.`,18,40,uc,.002],[`Study time to quiz score`,`A friendly positive trend with a meaningful baseline.`,6,52,uc,.005],[`Screen brightness to battery draw`,`A line where increasing input increases cost.`,.42,1.2,uc,.02],[`Discount to final price`,`A negative slope: more discount means lower price.`,-.8,100,uc,.004],[`Altitude to air temperature`,`A negative physical trend with an intercept.`,-3.5,70,uc,.006],[`Recipe servings to flour`,`A proportional recipe scaling example.`,120,0,fc,.02],[`Parking time to fee`,`A simple line with a starting fee and per-hour growth.`,3.5,4,uc,.012]],_c=Array.from({length:15},(e,t)=>{let n=[2e-4,5e-4,.001,.002,.004][t%5],r=1.2+t%3*.45;return hc({title:`Learning rate ${t+1}: ${n}`,category:`Learning rate`,summary:`Compare how step size changes convergence speed and stability.`,lesson:`A useful learning rate moves downhill visibly without bouncing across the valley.`,points:mc(r,8+t,{xs:dc,noise:t%2==0?0:2,seed:t}),defaultLearningRate:n})}),vc=Array.from({length:15},(e,t)=>{let n=t%2==1;return hc({title:`${n?`Outlier`:`Clean`} loss comparison ${t+1}`,category:`Loss functions`,summary:`Switch between MSE and MAE to see how error shape changes the update.`,lesson:`MSE squares large mistakes, so a single bad point can pull the fitted line harder than MAE.`,points:mc(2.4,12,{xs:uc,noise:.8+t%4*.4,seed:t+2,outlierIndex:n?7:void 0,outlierShift:n?22+t:0}),defaultLoss:n?`mae`:`mse`,defaultLearningRate:.008})}),yc=Array.from({length:15},(e,t)=>{let n=t%3==0?fc:t%3==1?uc:dc,r=t%3==0?`normalized`:t%3==1?`centered`:`wide`;return hc({title:`Feature scale ${t+1}: ${r}`,category:`Scaling`,summary:`The same visual idea becomes easier or harder to optimize depending on input scale.`,lesson:`Large input values make gradients large; normalized inputs usually tolerate larger learning rates.`,points:mc(1.1+t*.08,4,{xs:n,noise:.5,seed:t+4}),defaultLearningRate:r===`wide`?6e-4:.015})}),bc=Array.from({length:15},(e,t)=>{let n=.5+t*.5;return hc({title:`Noise level ${t+1}`,category:`Noise`,summary:`Watch the line chase a pattern when the points stop landing exactly on it.`,lesson:`Noise means the best line is not the line through every point; it is the line that balances errors.`,points:mc(3.1,-6,{noise:n,seed:t+6}),defaultLearningRate:.007})}),xc=Array.from({length:12},(e,t)=>{let n=t%2==0;return hc({title:`${n?`Curved data`:`Sparse data`} ${t+1}`,category:`Generalization`,summary:`Use a line even when the world is not perfectly linear.`,lesson:n?`A linear model can still be useful on curved data, but the residuals reveal its limits.`:`With only a few points, the line can look confident while still being fragile.`,points:mc(1.7,5,{xs:n?uc:[-8,-2,1,7],noise:.7,seed:t+8,curve:n?.12+t*.01:0}),defaultLearningRate:.007})}),Sc=[[`flipper_length_mm`,`body_mass_g`,`Flipper length to body mass`,`Longer flippers usually come with larger body mass.`],[`bill_length_mm`,`body_mass_g`,`Bill length to body mass`,`Bill length has signal, but the relationship is messier.`],[`bill_depth_mm`,`body_mass_g`,`Bill depth to body mass`,`A weak feature shows why not every measurement predicts well.`],[`flipper_length_mm`,`bill_length_mm`,`Flipper length to bill length`,`A moderate trend shows shared body-size information.`],[`bill_length_mm`,`bill_depth_mm`,`Bill length to bill depth`,`This relationship is noisy because species mix differently.`],[`year`,`body_mass_g`,`Observation year to body mass`,`A poor predictor is useful because the loss does not improve much.`]].flatMap(([e,t,n,r])=>[`MSE view`,`MAE view`,`small learning rate`].map((i,a)=>hc({title:`${n}: ${i}`,category:`Real data`,summary:`A checked-in CC0 CSV sample from Palmer Penguins, used without runtime network loading.`,lesson:r,xLabel:e.replaceAll(`_`,` `),yLabel:t.replaceAll(`_`,` `),points:ec(e,t),defaultLoss:a===1?`mae`:`mse`,defaultLearningRate:a===2?4e-7:1e-6,initialModel:{weight:0,bias:3e3,epoch:0},source:lc}))),Cc=[...gc.map(([e,t,n,r,i,a])=>hc({title:e,category:`Basics`,summary:t,lesson:`Start with simple lines so weight, bias, prediction, error, and loss become visible.`,xLabel:e===`Celsius to Fahrenheit`?`Celsius`:`Input`,yLabel:e===`Celsius to Fahrenheit`?`Fahrenheit`:`Target`,points:mc(n,r,{xs:i}),defaultLearningRate:a,initialModel:e===`Celsius to Fahrenheit`?{weight:.5,bias:.5,epoch:0}:void 0})),..._c,...vc,...yc,...bc,...xc,...Sc],wc=[`Basics`,`Learning rate`,`Loss functions`,`Scaling`,`Noise`,`Generalization`,`Real data`],Tc=[{x:-1,y:-1},{x:0,y:1},{x:1,y:3},{x:2,y:5}],Ec={weight:-.5,bias:0,step:0},Dc={weight:2,bias:1};function Oc(e,t){return t.weight*e.x+t.bias}function kc(e,t){if(e.length===0)throw Error(`meanSquaredError requires at least one point`);return e.reduce((e,n)=>{let r=Oc(n,t)-n.y;return e+r*r},0)/e.length}function Ac(e,t){if(e.length===0)throw Error(`analyticalGradient requires at least one point`);let n=2/e.length;return e.reduce((e,r)=>{let i=Oc(r,t)-r.y;return{weight:e.weight+n*i*r.x,bias:e.bias+n*i}},{weight:0,bias:0})}function jc(e,t,n){if(!(n>0)||!Number.isFinite(n))throw Error(`epsilon must be a positive finite number`);function r(r){let i={...t,[r]:t[r]+n},a={...t,[r]:t[r]-n};return(kc(e,i)-kc(e,a))/(2*n)}return{weight:r(`weight`),bias:r(`bias`)}}function Mc(e,t,n,r=1e-6){let i=Ac(e,t),a=jc(e,t,n),o={weight:Math.abs(i.weight-a.weight),bias:Math.abs(i.bias-a.bias)},s=[`weight`,`bias`].map(e=>{let t=Math.max(1,Math.abs(i[e]),Math.abs(a[e]));return o[e]/t}),c=Math.max(...s);return{analytical:i,numerical:a,absoluteError:o,maximumRelativeError:c,passes:c<=r}}function Nc(e,t,n){if(!Number.isInteger(n)||n<1)throw Error(`pointCount must be a positive integer`);if(e===`full-batch`)return Array.from({length:n},(e,t)=>t);if(e===`stochastic`)return[t%n];let r=t*2%n;return[r,(r+1)%n]}function Pc(e,t,n,r){if(!(n>0)||!Number.isFinite(n))throw Error(`learningRate must be a positive finite number`);let i=Nc(r,t.step,e.length),a=Ac(i.map(t=>e[t]),t),o={weight:t.weight-n*a.weight,bias:t.bias-n*a.bias,step:t.step+1};return{...o,loss:kc(e,o),batchIndices:i}}function Fc(e,t,n,r=Ec,i=Tc){if(!Number.isInteger(t)||t<0)throw Error(`steps must be a non-negative integer`);let a=[{...r,loss:kc(i,r),batchIndices:[]}],o=r;for(let r=0;r<t;r+=1){let t=Pc(i,o,n,e);a.push(t),o=t}return a}function Ic(e,t,n,r){if(!Number.isInteger(r)||r<2)throw Error(`resolution must be an integer of at least two`);let i=[];for(let a=0;a<r;a+=1){let o=n[0]+(n[1]-n[0])*(a/(r-1));for(let n=0;n<r;n+=1){let s=t[0]+(t[1]-t[0])*(n/(r-1));i.push({weight:s,bias:o,loss:kc(e,{weight:s,bias:o,step:0}),column:n,row:a})}}return i}var Lc=[{kind:`stochastic`,label:`SGD / 1 row`,summary:`Noisy, frequent updates`},{kind:`mini-batch`,label:`Mini-batch / 2 rows`,summary:`A compromise between noise and stability`},{kind:`full-batch`,label:`Full batch / 4 rows`,summary:`Stable average gradient`}],R={width:720,height:430,left:68,right:28,top:24,bottom:58,weightRange:[-1,3.5],biasRange:[-1,3],resolution:25};function Rc(e,t=5){return Math.abs(e)<1e-12?`0`:Math.abs(e)>=1e3||Math.abs(e)<1e-4?e.toExponential(3):Number(e.toFixed(t)).toString()}function zc(e){let t=R.width-R.left-R.right;return R.left+(e-R.weightRange[0])/(R.weightRange[1]-R.weightRange[0])*t}function Bc(e){let t=R.height-R.top-R.bottom;return R.top+(1-(e-R.biasRange[0])/(R.biasRange[1]-R.biasRange[0]))*t}function Vc(e,t){if(e.length===0)return``;let n=Math.max(e.length-1,1);return e.map((e,r)=>{let i=r/n*590,a=138-Math.log1p(e.loss)/t*138;return`${r===0?`M`:`L`} ${i.toFixed(2)} ${a.toFixed(2)}`}).join(` `)}function Hc(e,t){let n=Number(e);return Number.isFinite(n)?n:t}function Uc(){let[e,t]=(0,l.useState)(Ec),[n,r]=(0,l.useState)(1e-5),[i,a]=(0,l.useState)(.05),[o,s]=(0,l.useState)(20),c=(0,l.useMemo)(()=>Mc(Tc,e,n),[n,e]),u=(0,l.useMemo)(()=>Ic(Tc,R.weightRange,R.biasRange,R.resolution),[]),d=(0,l.useMemo)(()=>Math.max(...u.map(e=>Math.log1p(e.loss)),1),[u]),f=(0,l.useMemo)(()=>Lc.map(t=>({...t,trace:Fc(t.kind,o,i,e)})),[i,e,o]),p=Math.max(...f.flatMap(e=>e.trace.map(e=>Math.log1p(e.loss))),1),m=Pc(Tc,e,i,`full-batch`),h=(R.width-R.left-R.right)/R.resolution,g=(R.height-R.top-R.bottom)/R.resolution;function _(e,n){t(t=>({...t,[e]:Hc(n,t[e]),step:0}))}function v(){t(Ec),r(1e-5),a(.05),s(20)}return(0,E.jsxs)(`main`,{className:`workspace workspace--optimization`,children:[(0,E.jsxs)(`section`,{className:`optimization-stage`,"aria-label":`Optimization microscope`,children:[(0,E.jsxs)(`div`,{className:`lab-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Slope / check / step size / batch noise`}),(0,E.jsx)(`h2`,{children:`Optimization microscope`}),(0,E.jsx)(`p`,{children:`See the loss surface, verify the gradient independently, and compare three ways to choose training rows.`})]}),(0,E.jsxs)(`div`,{className:`lab-chip`,children:[`MSE `,Rc(kc(Tc,e),4)]})]}),(0,E.jsxs)(`section`,{className:`landscape-panel`,"aria-label":`Loss landscape`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Every location is one model`}),(0,E.jsx)(`h2`,{children:`Loss landscape`})]}),(0,E.jsx)(`span`,{children:`Darker = larger loss`})]}),(0,E.jsxs)(`svg`,{className:`landscape-chart`,viewBox:`0 0 ${R.width} ${R.height}`,role:`img`,"aria-label":`Mean squared error by weight and bias. Current weight ${Rc(e.weight)} and bias ${Rc(e.bias)}.`,children:[(0,E.jsx)(`title`,{children:`Loss landscape for a four-point linear regression problem`}),u.map(e=>(0,E.jsx)(`rect`,{className:`landscape-cell`,x:R.left+e.column*h,y:R.top+(R.resolution-1-e.row)*g,width:h+.4,height:g+.4,style:{opacity:.08+.78*(Math.log1p(e.loss)/d)}},`${e.row}-${e.column}`)),(0,E.jsx)(`line`,{className:`gradient-arrow`,x1:zc(e.weight),y1:Bc(e.bias),x2:zc(m.weight),y2:Bc(m.bias),markerEnd:`url(#gradient-arrow-head)`}),(0,E.jsx)(`defs`,{children:(0,E.jsx)(`marker`,{id:`gradient-arrow-head`,markerWidth:`8`,markerHeight:`8`,refX:`5`,refY:`3`,orient:`auto`,children:(0,E.jsx)(`path`,{d:`M 0 0 L 6 3 L 0 6 z`,className:`gradient-arrow-head`})})}),(0,E.jsx)(`circle`,{className:`optimum-point`,cx:zc(Dc.weight),cy:Bc(Dc.bias),r:`8`}),(0,E.jsx)(`text`,{className:`landscape-label`,x:zc(Dc.weight)+12,y:Bc(Dc.bias)-10,children:`minimum (2, 1)`}),(0,E.jsx)(`circle`,{className:`current-parameter-point`,cx:zc(e.weight),cy:Bc(e.bias),r:`9`}),(0,E.jsx)(`text`,{className:`landscape-label`,x:zc(e.weight)+12,y:Bc(e.bias)+22,children:`current model`}),(0,E.jsx)(`text`,{className:`axis-title`,x:R.width/2,y:R.height-10,children:`weight w`}),(0,E.jsx)(`text`,{className:`axis-title axis-title--optimization-y`,x:`18`,y:R.height/2,children:`bias b`})]}),(0,E.jsxs)(`div`,{className:`landscape-equation`,children:[(0,E.jsxs)(`code`,{children:[`w' = `,Rc(e.weight),` - `,Rc(i),` x (`,Rc(c.analytical.weight),`) = `,Rc(m.weight)]}),(0,E.jsxs)(`code`,{children:[`b' = `,Rc(e.bias),` - `,Rc(i),` x (`,Rc(c.analytical.bias),`) = `,Rc(m.bias)]})]})]}),(0,E.jsxs)(`section`,{className:`gradient-check-panel`,"aria-label":`Finite-difference gradient check`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Backpropagation gets an independent audit`}),(0,E.jsx)(`h2`,{children:`Finite-difference gradient check`})]}),(0,E.jsx)(`span`,{className:c.passes?`check-status check-status--pass`:`check-status check-status--fail`,children:c.passes?`PASS`:`CHECK EPSILON`})]}),(0,E.jsxs)(`div`,{className:`gradient-check-grid`,role:`table`,"aria-label":`Gradient comparison`,children:[(0,E.jsx)(`span`,{role:`columnheader`,children:`Parameter`}),(0,E.jsx)(`span`,{role:`columnheader`,children:`Backprop`}),(0,E.jsx)(`span`,{role:`columnheader`,children:`Finite difference`}),(0,E.jsx)(`span`,{role:`columnheader`,children:`Absolute error`}),(0,E.jsx)(`strong`,{role:`cell`,children:`weight`}),(0,E.jsx)(`code`,{role:`cell`,children:Rc(c.analytical.weight)}),(0,E.jsx)(`code`,{role:`cell`,children:Rc(c.numerical.weight)}),(0,E.jsx)(`code`,{role:`cell`,children:Rc(c.absoluteError.weight)}),(0,E.jsx)(`strong`,{role:`cell`,children:`bias`}),(0,E.jsx)(`code`,{role:`cell`,children:Rc(c.analytical.bias)}),(0,E.jsx)(`code`,{role:`cell`,children:Rc(c.numerical.bias)}),(0,E.jsx)(`code`,{role:`cell`,children:Rc(c.absoluteError.bias)})]}),(0,E.jsx)(`p`,{children:`Finite differences nudge one parameter by +/- epsilon and estimate the slope without using backpropagation.`})]}),(0,E.jsxs)(`section`,{className:`batch-comparison-panel`,"aria-label":`Batch strategy comparison`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same model / same data / different row selection`}),(0,E.jsx)(`h2`,{children:`Batch versus stochastic updates`})]}),(0,E.jsxs)(`span`,{children:[o,` updates`]})]}),(0,E.jsxs)(`svg`,{className:`batch-chart`,viewBox:`0 0 650 175`,role:`img`,"aria-label":`Loss trajectories for stochastic, mini-batch, and full-batch gradient descent`,children:[(0,E.jsx)(`line`,{className:`batch-grid`,x1:`42`,x2:`632`,y1:`148`,y2:`148`}),(0,E.jsx)(`line`,{className:`batch-grid`,x1:`42`,x2:`42`,y1:`10`,y2:`148`}),(0,E.jsx)(`g`,{transform:`translate(42 10)`,children:f.map(e=>(0,E.jsx)(`path`,{className:`batch-line batch-line--${e.kind}`,d:Vc(e.trace,p)},e.kind))}),(0,E.jsx)(`text`,{className:`batch-axis-label`,x:`337`,y:`172`,children:`update`}),(0,E.jsx)(`text`,{className:`batch-axis-label batch-axis-label--y`,x:`12`,y:`82`,children:`log loss`})]}),(0,E.jsx)(`div`,{className:`strategy-grid`,children:f.map(e=>{let t=e.trace[e.trace.length-1];return(0,E.jsxs)(`div`,{className:`strategy-summary strategy-summary--${e.kind}`,children:[(0,E.jsx)(`strong`,{children:e.label}),(0,E.jsx)(`span`,{children:e.summary}),(0,E.jsxs)(`code`,{children:[`loss `,Rc(t.loss,4)]}),(0,E.jsxs)(`small`,{children:[`w `,Rc(t.weight,3),` / b `,Rc(t.bias,3)]})]},e.kind)})})]})]}),(0,E.jsxs)(`aside`,{className:`controls optimization-controls`,"aria-label":`Optimization controls`,children:[(0,E.jsxs)(`div`,{className:`lesson`,children:[(0,E.jsx)(`span`,{children:`Try this`}),(0,E.jsx)(`p`,{children:`Move the model away from the minimum, then increase the learning rate until one or more trajectories overshoot.`})]}),(0,E.jsxs)(`div`,{className:`field-grid`,children:[(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Weight w`}),(0,E.jsx)(`input`,{"aria-label":`Optimization weight`,type:`number`,step:`0.1`,value:e.weight,onChange:e=>_(`weight`,e.target.value)})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Bias b`}),(0,E.jsx)(`input`,{"aria-label":`Optimization bias`,type:`number`,step:`0.1`,value:e.bias,onChange:e=>_(`bias`,e.target.value)})]})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Learning rate`}),(0,E.jsx)(`input`,{"aria-label":`Optimization learning rate`,type:`range`,min:`0.005`,max:`0.3`,step:`0.005`,value:i,onChange:e=>a(Number(e.target.value))}),(0,E.jsx)(`input`,{type:`number`,min:`0.005`,max:`0.3`,step:`0.005`,value:i,onChange:e=>a(Hc(e.target.value,i))})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Comparison updates`}),(0,E.jsx)(`input`,{"aria-label":`Comparison updates`,type:`range`,min:`1`,max:`80`,step:`1`,value:o,onChange:e=>s(Number(e.target.value))}),(0,E.jsx)(`strong`,{children:o})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Finite-difference epsilon`}),(0,E.jsxs)(`select`,{"aria-label":`Finite-difference epsilon`,value:n,onChange:e=>r(Number(e.target.value)),children:[(0,E.jsx)(`option`,{value:`0.01`,children:`1e-2`}),(0,E.jsx)(`option`,{value:`0.001`,children:`1e-3`}),(0,E.jsx)(`option`,{value:`0.0001`,children:`1e-4`}),(0,E.jsx)(`option`,{value:`0.00001`,children:`1e-5`}),(0,E.jsx)(`option`,{value:`0.000001`,children:`1e-6`}),(0,E.jsx)(`option`,{value:`1e-8`,children:`1e-8`})]})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Maximum relative gradient error`}),(0,E.jsx)(`strong`,{children:Rc(c.maximumRelativeError)})]}),(0,E.jsx)(`button`,{type:`button`,onClick:v,children:`Reset optimization lab`})]})]})}var Wc={schema_version:1,id:`tiny-affine-precision-residency`,title:`Two close inputs through one affine neuron`,question:`Which digits and transfers disappear when precision shrinks and buffers stay resident?`,graph:{equation:`y = x * w + b`,weight:2,bias:0},scenario:{inputs:[1.0004,1.0006],reference_outputs:[2.0008,2.0012]},formats:[{id:`binary32`,title:`IEEE-754 binary32`,storage_bytes_per_value:4,input_payload_file:`../payloads/00-input-x.f32le.hex`,output_payload_file:`../payloads/00-output-y.f32le.hex`,encoded_inputs:[1.0003999471664429,1.000599980354309],encoded_weight:2,accumulators:[2.0007998943328857,2.001199960708618],outputs:[2.0007998943328857,2.001199960708618],maximum_absolute_error:1.056671141697052e-7},{id:`binary16`,title:`IEEE-754 binary16`,storage_bytes_per_value:2,input_payload_file:`../payloads/00-input-x.f16le.hex`,output_payload_file:`../payloads/00-output-y.f16le.hex`,encoded_inputs:[1,1.0009765625],encoded_weight:2,accumulators:[2,2.001953125],outputs:[2,2.001953125],maximum_absolute_error:.0007999999999999119},{id:`symmetric_int8`,title:`Symmetric signed int8`,storage_bytes_per_value:1,accumulator_storage_bytes:4,input_payload_file:`../payloads/00-input-x.i8.hex`,weight_payload_file:`../payloads/00-weight-w.i8.hex`,input_scale:.01,weight_scale:.5,zero_point:0,encoded_inputs:[100,100],encoded_weight:4,accumulators:[400,400],outputs:[2,2],maximum_absolute_error:.0011999999999998678}],residency:{dtype:`binary32`,repeat_count:3,upload_bytes_per_copy:16,download_bytes_per_copy:8,strategies:[{id:`eager`,title:`Eager copies`,steps:[`upload x, w, and b`,`run affine neuron`,`download y`,`discard device buffers`],upload_count:3,download_count:3,total_transfer_bytes:72},{id:`resident`,title:`Resident buffers`,steps:[`upload x, w, and b once`,`run affine neuron three times`,`keep x, w, b, and y on device`,`download final y once`],upload_count:1,download_count:1,total_transfer_bytes:24}]}},Gc=512,Kc=1e6,qc=[`binary32`,`binary16`,`symmetric_int8`],Jc=[`eager`,`resident`],Yc={binary32:`IEEE-754 binary32`,binary16:`IEEE-754 binary16`,symmetric_int8:`Symmetric signed int8`},Xc={binary32:`../payloads/00-input-x.f32le.hex`,binary16:`../payloads/00-input-x.f16le.hex`,symmetric_int8:`../payloads/00-input-x.i8.hex`},Zc={eager:`Eager copies`,resident:`Resident buffers`},Qc={eager:[`upload x, w, and b`,`run affine neuron`,`download y`,`discard device buffers`],resident:[`upload x, w, and b once`,`run affine neuron three times`,`keep x, w, b, and y on device`,`download final y once`]};function $c(e,t,n){if(typeof e!=`object`||!e||Array.isArray(e))throw Error(`${n} must be an object`);let r=Object.keys(e).sort(),i=[...t].sort();if(r.join(`,`)!==i.join(`,`))throw Error(`${n} has unexpected fields`);return e}function el(e,t){if(typeof e!=`string`||e.length<1||e.length>Gc)throw Error(`${t} must be bounded text`);return e}function tl(e,t){if(typeof e!=`number`||!Number.isFinite(e)||Math.abs(e)>Kc)throw Error(`${t} must be a finite bounded number`);return e}function nl(e,t,n,r){let i=tl(e,r);if(!Number.isInteger(i)||i<t||i>n)throw Error(`${r} must be a bounded integer`);return i}function rl(e,t,n){if(!Array.isArray(e)||e.length!==t)throw Error(`${n} must contain ${t} numbers`);return e.map((e,t)=>tl(e,`${n}[${t}]`))}function il(e,t,n){if(!Array.isArray(e)||e.length!==t)throw Error(`${n} must contain ${t} strings`);return e.map((e,t)=>el(e,`${n}[${t}]`))}function al(e,t,n){if(e.length!==t.length||e.some((e,n)=>e!==t[n]))throw Error(`${n} does not match the arithmetic oracle`)}function ol(e){let t=Math.floor(e),n=e-t;return n<.5?t:n>.5?t+1:t%2==0?t:t+1}function sl(e){if(!Number.isFinite(e))throw Error(`binary16 input must be finite and representable`);if(e===0)return e;let t=e<0?-1:1,n=Math.abs(e),r=n<2**-14?ol(n/2**-24)*2**-24:(()=>{let e=2**(Math.floor(Math.log2(n))-10);return ol(n/e)*e})();if(r>65504)throw Error(`binary16 input must be finite and representable`);return t*r}function cl(e,t,n,r){let i=e.map(r),a=r(t),o=r(n),s=i.map(e=>r(e*a));return{encodedInputs:i,encodedWeight:a,accumulators:s,outputs:s.map(e=>r(e+o))}}function ll(e,t){return Math.max(...e.map((e,n)=>Math.abs(e-t[n])))}function ul(e,t,n,r,i,a){let o=$c(e,t===`symmetric_int8`?[`id`,`title`,`storage_bytes_per_value`,`input_payload_file`,`weight_payload_file`,`accumulator_storage_bytes`,`input_scale`,`weight_scale`,`zero_point`,`encoded_inputs`,`encoded_weight`,`accumulators`,`outputs`,`maximum_absolute_error`]:[`id`,`title`,`storage_bytes_per_value`,`input_payload_file`,`output_payload_file`,`encoded_inputs`,`encoded_weight`,`accumulators`,`outputs`,`maximum_absolute_error`],`format ${t}`);if(o.id!==t)throw Error(`precision format roster is not canonical`);if(el(o.title,`format title`)!==Yc[t])throw Error(`precision format title is not canonical`);if(o.input_payload_file!==Xc[t])throw Error(`input payload reference is not canonical`);if(t===`binary32`&&o.output_payload_file!==`../payloads/00-output-y.f32le.hex`||t===`binary16`&&o.output_payload_file!==`../payloads/00-output-y.f16le.hex`)throw Error(`output payload reference is not canonical`);if(t===`symmetric_int8`&&o.weight_payload_file!==`../payloads/00-weight-w.i8.hex`)throw Error(`weight payload reference is not canonical`);let s=rl(o.encoded_inputs,2,`encoded inputs`),c=tl(o.encoded_weight,`encoded weight`),l=rl(o.accumulators,2,`accumulators`),u=rl(o.outputs,2,`outputs`),d,f,p,m,h;if(t===`symmetric_int8`){if(f=tl(o.input_scale,`input scale`),p=tl(o.weight_scale,`weight scale`),m=nl(o.zero_point,-128,127,`zero point`),h=nl(o.accumulator_storage_bytes,1,8,`accumulator storage width`),f!==.01||p!==.5||m!==0||h!==4)throw Error(`int8 quantization parameters are not canonical`);let e=n.map(e=>ol(e/f)),t=ol(r/p),i=e.map(e=>e*t);d={encodedInputs:e,encodedWeight:t,accumulators:i,outputs:i.map(e=>e*f*p)}}else d=cl(n,r,i,t===`binary32`?Math.fround:sl);if(al(s,d.encodedInputs,`encoded inputs`),c!==d.encodedWeight)throw Error(`encoded weight does not match the arithmetic oracle`);al(l,d.accumulators,`accumulators`),al(u,d.outputs,`outputs`);let g=tl(o.maximum_absolute_error,`maximum absolute error`);if(g!==ll(u,a))throw Error(`maximum absolute error does not match the arithmetic oracle`);let _=nl(o.storage_bytes_per_value,1,8,`storage width`);if(_!==(t===`binary32`?4:t===`binary16`?2:1))throw Error(`storage width is not canonical`);return{id:t,title:Yc[t],storageBytesPerValue:_,encodedInputs:s,encodedWeight:c,accumulators:l,outputs:u,maximumAbsoluteError:g,...h===void 0?{}:{accumulatorStorageBytes:h},...f===void 0?{}:{inputScale:f,weightScale:p,zeroPoint:m}}}function dl(e){let t=$c(e,[`schema_version`,`id`,`title`,`question`,`graph`,`scenario`,`formats`,`residency`],`precision fixture`);if(t.schema_version!==1||t.id!==`tiny-affine-precision-residency`)throw Error(`precision fixture identity is not canonical`);let n=$c(t.graph,[`equation`,`weight`,`bias`],`graph`),r=tl(n.weight,`weight`),i=tl(n.bias,`bias`);if(n.equation!==`y = x * w + b`||r!==2||i!==0)throw Error(`affine graph is not canonical`);let a=$c(t.scenario,[`inputs`,`reference_outputs`],`scenario`),o=rl(a.inputs,2,`inputs`),s=rl(a.reference_outputs,2,`reference outputs`);if(o.join(`,`)!==`1.0004,1.0006`)throw Error(`input scenario is not canonical`);al(s,o.map(e=>e*r+i),`reference outputs`);let c=t.formats;if(!Array.isArray(c)||c.length!==3)throw Error(`precision fixture must contain three formats`);let l=qc.map((e,t)=>ul(c[t],e,o,r,i,s)),u=$c(t.residency,[`dtype`,`repeat_count`,`upload_bytes_per_copy`,`download_bytes_per_copy`,`strategies`],`residency`),d=nl(u.repeat_count,1,16,`repeat count`),f=nl(u.upload_bytes_per_copy,1,1024,`upload bytes`),p=nl(u.download_bytes_per_copy,1,1024,`download bytes`);if(u.dtype!==`binary32`||d!==3||f!==16||p!==8)throw Error(`residency byte contract is not canonical`);let m=u.strategies;if(!Array.isArray(m)||m.length!==2)throw Error(`residency strategy roster is not canonical`);let h=Jc.map((e,t)=>{let n=$c(m[t],[`id`,`title`,`steps`,`upload_count`,`download_count`,`total_transfer_bytes`],`strategy ${e}`),r=e===`eager`?d:1,i=e===`eager`?d:1,a=e===`eager`?(f+p)*d:f+p,o=il(n.steps,4,`strategy steps`);if(n.id!==e||n.title!==Zc[e]||o.join(`\0`)!==Qc[e].join(`\0`)||n.upload_count!==r||n.download_count!==i||n.total_transfer_bytes!==a)throw Error(`residency transfer oracle is not canonical`);return{id:e,title:Zc[e],steps:o,uploadCount:r,downloadCount:i,totalTransferBytes:a}});return ml({id:`tiny-affine-precision-residency`,title:el(t.title,`fixture title`),question:el(t.question,`fixture question`),graph:{equation:`y = x * w + b`,weight:r,bias:i},scenario:{inputs:o,referenceOutputs:s},formats:l,residency:{dtype:`binary32`,repeatCount:d,uploadBytesPerCopy:f,downloadBytesPerCopy:p,strategies:h}})}var fl=dl(Wc);function pl(e=`binary16`,t=`resident`,n=fl.residency.repeatCount){if(!Number.isInteger(n)||n<1||n>8)throw Error(`repeat count must be an integer from 1 through 8`);let r=fl,i=r.formats.find(t=>t.id===e),a=r.residency.strategies.find(e=>e.id===t);if(i===void 0||a===void 0)throw Error(`unknown precision or residency selection`);let o=r.scenario.inputs.map((e,t)=>({input:e,encodedInput:i.encodedInputs[t],encodedWeight:i.encodedWeight,accumulator:i.accumulators[t],output:i.outputs[t],referenceOutput:r.scenario.referenceOutputs[t],absoluteError:Math.abs(i.outputs[t]-r.scenario.referenceOutputs[t])})),s=(r.residency.uploadBytesPerCopy+r.residency.downloadBytesPerCopy)*n,c=t===`eager`?n:1,l=t===`eager`?n:1,u=t===`eager`?s:r.residency.uploadBytesPerCopy+r.residency.downloadBytesPerCopy;return ml({fixture:r,format:i,strategy:ml({...a,steps:a.steps.map((e,t)=>t===1?`run affine neuron ${n} ${n===1?`time`:`times`}`:e),uploadCount:c,downloadCount:l,totalTransferBytes:u}),rows:o,repeatCount:n,uploadCount:c,downloadCount:l,transferBytes:u,bytesSavedAgainstEager:s-u})}function ml(e){return typeof e!=`object`||!e||Object.isFrozen(e)?e:(Object.freeze(e),Object.values(e).forEach(e=>ml(e)),e)}function hl(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):Number(e.toPrecision(10)).toString()}function gl(e,t,n){return e===`symmetric_int8`?`${t}-byte operands · ${n}-byte accumulator`:`${t} byte${t===1?``:`s`} / value`}function _l(){let[e,t]=(0,l.useState)(`binary16`),[n,r]=(0,l.useState)(`resident`),[i,a]=(0,l.useState)(3),o=(0,l.useMemo)(()=>pl(e,n,i),[e,n,i]);return(0,E.jsxs)(`main`,{className:`workspace workspace--precision-residency`,children:[(0,E.jsxs)(`section`,{className:`precision-stage`,children:[(0,E.jsxs)(`header`,{className:`precision-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN32 - smaller numbers, fewer journeys`}),(0,E.jsx)(`h2`,{children:`Precision and residency laboratory`}),(0,E.jsx)(`p`,{children:o.fixture.question})]}),(0,E.jsx)(`span`,{className:`precision-chip`,children:gl(o.format.id,o.format.storageBytesPerValue,o.format.accumulatorStorageBytes)})]}),(0,E.jsxs)(`section`,{className:`precision-paper`,"aria-label":`Reference affine calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`1 - calculate`}),(0,E.jsx)(`h2`,{children:`Start with the paper answer`})]}),(0,E.jsx)(`code`,{children:`y = x * 2 + 0`})]}),(0,E.jsxs)(`div`,{className:`precision-equation-row`,children:[(0,E.jsxs)(`code`,{children:[`1.0004 * 2 = `,(0,E.jsx)(`strong`,{children:`2.0008`})]}),(0,E.jsxs)(`code`,{children:[`1.0006 * 2 = `,(0,E.jsx)(`strong`,{children:`2.0012`})]})]})]}),(0,E.jsxs)(`section`,{className:`precision-formats`,"aria-label":`Precision and quantization formats`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`2 - encode`}),(0,E.jsx)(`h2`,{children:`Move onto a smaller number grid`})]}),(0,E.jsxs)(`code`,{children:[`max error `,o.format.maximumAbsoluteError.toExponential(3)]})]}),(0,E.jsx)(`div`,{className:`precision-format-buttons`,children:o.fixture.formats.map(n=>(0,E.jsxs)(`button`,{"aria-label":`Use ${n.title}`,"aria-pressed":n.id===e,onClick:()=>t(n.id),type:`button`,children:[(0,E.jsx)(`small`,{children:gl(n.id,n.storageBytesPerValue,n.accumulatorStorageBytes)}),(0,E.jsx)(`strong`,{children:n.title}),(0,E.jsxs)(`code`,{children:[`[`,n.outputs.map(hl).join(`, `),`]`]})]},n.id))}),o.format.id===`symmetric_int8`?(0,E.jsxs)(`p`,{className:`precision-scale-note`,children:[(0,E.jsxs)(`code`,{children:[`input scale `,o.format.inputScale]}),(0,E.jsxs)(`code`,{children:[`weight scale `,o.format.weightScale]}),(0,E.jsx)(`span`,{children:`Both close inputs become integer 100.`})]}):null]}),(0,E.jsxs)(`section`,{className:`precision-trace`,"aria-label":`Selected precision arithmetic trace`,children:[(0,E.jsx)(`div`,{className:`panel-heading`,children:(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`3 - inspect `,o.format.id]}),(0,E.jsx)(`h2`,{children:`Every rounding step stays visible`})]})}),(0,E.jsxs)(`div`,{className:`precision-table`,role:`table`,"aria-label":`Precision output and error rows`,children:[(0,E.jsxs)(`div`,{className:`precision-table-head`,role:`row`,children:[(0,E.jsx)(`strong`,{role:`columnheader`,children:`paper x`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`encoded x`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`encoded w`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`accumulator`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`output`}),(0,E.jsx)(`strong`,{role:`columnheader`,children:`absolute error`})]}),o.rows.map((e,t)=>(0,E.jsxs)(`div`,{role:`row`,children:[(0,E.jsx)(`code`,{role:`cell`,children:hl(e.input)}),(0,E.jsx)(`code`,{role:`cell`,children:hl(e.encodedInput)}),(0,E.jsx)(`code`,{role:`cell`,children:hl(e.encodedWeight)}),(0,E.jsx)(`code`,{role:`cell`,children:hl(e.accumulator)}),(0,E.jsx)(`code`,{role:`cell`,children:hl(e.output)}),(0,E.jsx)(`code`,{role:`cell`,children:e.absoluteError.toExponential(3)})]},t))]})]}),(0,E.jsxs)(`section`,{className:`precision-residency`,"aria-label":`Buffer residency transfer trace`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`4 - place buffers`}),(0,E.jsx)(`h2`,{children:`Same answer, fewer boundary crossings`})]}),(0,E.jsxs)(`code`,{children:[o.fixture.residency.dtype,` baseline · `,o.transferBytes,` transfer bytes`]})]}),(0,E.jsxs)(`div`,{className:`precision-transfer-flow`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`host to device`}),(0,E.jsxs)(`strong`,{children:[o.uploadCount,` upload`,o.uploadCount===1?``:`s`]}),(0,E.jsxs)(`code`,{children:[o.fixture.residency.uploadBytesPerCopy,` bytes each`]})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`device work`}),(0,E.jsxs)(`strong`,{children:[o.repeatCount,` forward pass`,o.repeatCount===1?``:`es`]}),(0,E.jsx)(`code`,{children:`x, w, b, y`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`device to host`}),(0,E.jsxs)(`strong`,{children:[o.downloadCount,` download`,o.downloadCount===1?``:`s`]}),(0,E.jsxs)(`code`,{children:[o.fixture.residency.downloadBytesPerCopy,` bytes each`]})]})]}),(0,E.jsxs)(`div`,{className:`precision-transfer-equation`,children:[(0,E.jsx)(`code`,{children:n===`eager`?`(${o.fixture.residency.uploadBytesPerCopy} + ${o.fixture.residency.downloadBytesPerCopy}) * ${o.repeatCount}`:`${o.fixture.residency.uploadBytesPerCopy} + ${o.fixture.residency.downloadBytesPerCopy}`}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`strong`,{children:[o.transferBytes,` bytes`]}),(0,E.jsx)(`span`,{children:`-`}),(0,E.jsxs)(`code`,{children:[o.bytesSavedAgainstEager,` bytes saved vs eager`]})]})]})]}),(0,E.jsxs)(`aside`,{className:`precision-controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Experiment controls`}),(0,E.jsx)(`h2`,{children:`Separate representation from travel`}),(0,E.jsxs)(`label`,{children:[`repeat forward pass`,(0,E.jsx)(`input`,{"aria-label":`Forward pass repeats`,max:`8`,min:`1`,onInput:e=>a(Number(e.currentTarget.value)),type:`range`,value:i}),(0,E.jsx)(`code`,{children:i})]}),(0,E.jsx)(`div`,{className:`precision-strategy-buttons`,children:o.fixture.residency.strategies.map(e=>(0,E.jsxs)(`button`,{"aria-label":e.title,"aria-pressed":e.id===n,onClick:()=>r(e.id),type:`button`,children:[(0,E.jsx)(`strong`,{children:e.title}),(0,E.jsx)(`span`,{children:e.steps[0]})]},e.id))}),(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Transfer accounting`}),(0,E.jsx)(`p`,{children:`The copy experiment stays on a binary32 byte baseline so number representation and buffer travel can be changed independently.`})]}),(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Selected schedule`}),(0,E.jsx)(`ol`,{children:o.strategy.steps.map(e=>(0,E.jsx)(`li`,{children:e},e))})]}),(0,E.jsxs)(`section`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Rust-core direction`}),(0,E.jsx)(`p`,{children:`Keep byte order, scales, ownership, and explicit downloads in a future C ABI. The fixture is ready before that ABI exists.`})]}),(0,E.jsxs)(`section`,{className:`precision-warning`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Measure, do not assume`}),(0,E.jsx)(`p`,{children:`Smaller values and fewer copies are performance hypotheses. Accuracy and timing still need workload-specific tests.`})]})]})]})}var vl=[1,2,0],yl={inputWeight:2,recurrentWeight:.5,bias:-1},bl=.1,xl=1e-6;function Sl(e){return Math.abs(e)<1e-12?0:e}function Cl(e=vl,t=0,n=yl,r=!0){if(e.length!==3||![...e,t,n.inputWeight,n.recurrentWeight,n.bias].every(Number.isFinite))throw Error(`NN09 V1 needs three finite inputs, state, and parameters.`);let i=t,a=e.map((e,t)=>{let a=Sl(n.inputWeight*e),o=r?Sl(n.recurrentWeight*i):0,s=Sl(a+o+n.bias),c=Sl(Math.max(0,s)),l={time:t,input:e,previousState:i,inputProduct:a,recurrentProduct:o,bias:n.bias,preactivation:s,state:c};return i=c,l});return{steps:a,states:a.map(e=>e.state),finalState:a[a.length-1].state}}function wl(e,t,n,r){return .5*(Cl(e,t,n).finalState-r)**2}function Tl(e,t,n,r,i,a){let o={...r,[e]:r[e]+a},s={...r,[e]:r[e]-a};return(wl(t,n,o,i)-wl(t,n,s,i))/(2*a)}function El(e=vl,t=0,n=yl,r=0,i=bl,a=xl){if(![r,i,a].every(Number.isFinite)||a<=0)throw Error(`NN10 V1 needs a finite target and learning rate, plus a positive epsilon.`);let o=Cl(e,t,n),s=.5*(o.finalState-r)**2,c=0,l=[];for(let e=o.steps.length-1;e>=0;--e){let t=o.steps[e],i=e===o.steps.length-1?o.finalState-r:0,a=Sl(i+c),s=+(t.preactivation>0),u=Sl(a*s),d={inputWeight:Sl(u*t.input),recurrentWeight:Sl(u*t.previousState),bias:u},f=Sl(u*n.recurrentWeight);l.push({time:e,directStateGradient:i,futureStateGradient:c,stateGradient:a,reluDerivative:s,preactivationGradient:u,parameterContributions:d,previousStateGradient:f}),c=f}let u=l.reduce((e,t)=>({inputWeight:Sl(e.inputWeight+t.parameterContributions.inputWeight),recurrentWeight:Sl(e.recurrentWeight+t.parameterContributions.recurrentWeight),bias:Sl(e.bias+t.parameterContributions.bias),initialState:t.time===0?t.previousStateGradient:e.initialState}),{inputWeight:0,recurrentWeight:0,bias:0,initialState:0}),d={inputWeight:Tl(`inputWeight`,e,t,n,r,a),recurrentWeight:Tl(`recurrentWeight`,e,t,n,r,a),bias:Tl(`bias`,e,t,n,r,a)},f={inputWeight:Math.abs(u.inputWeight-d.inputWeight),recurrentWeight:Math.abs(u.recurrentWeight-d.recurrentWeight),bias:Math.abs(u.bias-d.bias)},p={inputWeight:Sl(n.inputWeight-i*u.inputWeight),recurrentWeight:Sl(n.recurrentWeight-i*u.recurrentWeight),bias:Sl(n.bias-i*u.bias)},m=Cl(e,t,p);return{forward:o,target:r,loss:s,backwardSteps:l,gradientTotals:u,numericalGradients:d,gradientErrors:f,maxGradientError:Math.max(...Object.values(f)),update:{learningRate:i,parameters:p,preactivations:m.steps.map(e=>e.preactivation),states:m.states,loss:.5*(m.finalState-r)**2}}}function z(e){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-6?e.toExponential(2):Number(e.toFixed(6)).toString()}function Dl({onShowForward:e,onShowGates:t}){let n=(0,l.useMemo)(()=>El(),[]),[r,i]=(0,l.useState)(2),a=n.backwardSteps.find(e=>e.time===r),o=[...n.backwardSteps].reverse();return(0,E.jsxs)(`main`,{className:`workspace workspace--bptt`,children:[(0,E.jsxs)(`section`,{className:`bptt-stage`,"aria-label":`Backpropagation through time trace`,children:[(0,E.jsxs)(`div`,{className:`bptt-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN10 · sequence gradients`}),(0,E.jsx)(`h2`,{children:`Backpropagation-through-time microscope`}),(0,E.jsx)(`p`,{children:`Keep the three saved forward states, then reverse every arrow. Watch later evidence reach earlier cells and add into one shared gradient.`})]}),(0,E.jsxs)(`div`,{className:`bptt-loss-chip`,children:[(0,E.jsx)(`small`,{children:`final-state loss`}),(0,E.jsx)(`strong`,{children:z(n.loss)})]})]}),(0,E.jsxs)(`section`,{className:`bptt-panel`,"aria-label":`Forward states and backward gradient lane`,children:[(0,E.jsxs)(`div`,{className:`bptt-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Forward saved · backward reversed`}),(0,E.jsx)(`h2`,{children:`One chain, two directions`})]}),(0,E.jsxs)(`code`,{children:[`target = `,z(n.target)]})]}),(0,E.jsxs)(`div`,{className:`bptt-forward-lane`,"aria-label":`Saved forward states`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`initial`}),(0,E.jsx)(`strong`,{children:`h[-1] = 0`})]}),n.forward.steps.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`a[`,e.time,`] = `,z(e.preactivation)]}),(0,E.jsxs)(`strong`,{children:[`h[`,e.time,`] = `,z(e.state)]})]},e.time)),(0,E.jsxs)(`div`,{className:`bptt-forward-lane__loss`,children:[(0,E.jsx)(`small`,{children:`half-squared`}),(0,E.jsxs)(`strong`,{children:[`L = `,z(n.loss)]})]})]}),(0,E.jsxs)(`div`,{className:`bptt-direction-label`,children:[(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`←`}),`backward pass runs from t = 2 to t = 0`]}),(0,E.jsx)(`div`,{className:`bptt-backward-lane`,"aria-label":`Reverse-time gradient steps`,children:n.backwardSteps.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Select backward step ${e.time}`,"aria-pressed":r===e.time,className:r===e.time?`bptt-step bptt-step--active`:`bptt-step`,type:`button`,onClick:()=>i(e.time),children:[(0,E.jsxs)(`small`,{children:[`reverse t = `,e.time]}),(0,E.jsxs)(`strong`,{children:[`dL/dh = `,z(e.stateGradient)]}),(0,E.jsxs)(`span`,{children:[`dL/da = `,z(e.preactivationGradient)]})]},e.time))}),(0,E.jsxs)(`div`,{className:`bptt-arithmetic`,"aria-label":`Selected backward arithmetic`,children:[(0,E.jsxs)(`div`,{className:`bptt-arithmetic-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected · reverse step `,r]}),(0,E.jsx)(`h3`,{children:`Combine incoming gradient before differentiating`})]}),(0,E.jsxs)(`code`,{children:[`ReLU' = `,z(a.reluDerivative)]})]}),(0,E.jsxs)(`div`,{className:`bptt-equation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`direct loss`}),(0,E.jsx)(`strong`,{children:z(a.directStateGradient)})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`from future`}),(0,E.jsx)(`strong`,{children:z(a.futureStateGradient)})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`dL/dh[`,r,`]`]}),(0,E.jsx)(`strong`,{children:z(a.stateGradient)})]}),(0,E.jsx)(`span`,{children:`×`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`ReLU derivative`}),(0,E.jsx)(`strong`,{children:z(a.reluDerivative)})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{className:`bptt-equation__result`,children:[(0,E.jsxs)(`small`,{children:[`dL/da[`,r,`]`]}),(0,E.jsx)(`strong`,{children:z(a.preactivationGradient)})]})]}),(0,E.jsxs)(`div`,{className:`bptt-local-gradients`,children:[(0,E.jsxs)(`code`,{children:[`ΔW_x = `,z(a.parameterContributions.inputWeight)]}),(0,E.jsxs)(`code`,{children:[`ΔW_h = `,z(a.parameterContributions.recurrentWeight)]}),(0,E.jsxs)(`code`,{children:[`Δb = `,z(a.parameterContributions.bias)]}),(0,E.jsxs)(`code`,{children:[`to h[`,r-1,`] = `,z(a.previousStateGradient)]})]})]})]}),(0,E.jsxs)(`section`,{className:`bptt-panel`,"aria-label":`Shared gradient reduction`,children:[(0,E.jsxs)(`div`,{className:`bptt-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Three executions · one parameter set`}),(0,E.jsx)(`h2`,{children:`Shared gradients add; they do not overwrite`})]}),(0,E.jsx)(`strong`,{className:`bptt-pass`,children:`ACCUMULATE`})]}),(0,E.jsx)(`div`,{className:`bptt-table-wrap`,children:(0,E.jsxs)(`table`,{className:`bptt-table`,children:[(0,E.jsx)(`caption`,{children:`Per-time-step parameter contributions and their totals`}),(0,E.jsx)(`thead`,{children:(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`col`,children:`gradient`}),o.map(e=>(0,E.jsxs)(`th`,{scope:`col`,children:[`t = `,e.time]},e.time)),(0,E.jsx)(`th`,{scope:`col`,children:`total`})]})}),(0,E.jsxs)(`tbody`,{children:[(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`dL/dW_x`}),o.map(e=>(0,E.jsx)(`td`,{children:z(e.parameterContributions.inputWeight)},e.time)),(0,E.jsx)(`td`,{children:(0,E.jsx)(`strong`,{children:z(n.gradientTotals.inputWeight)})})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`dL/dW_h`}),o.map(e=>(0,E.jsx)(`td`,{children:z(e.parameterContributions.recurrentWeight)},e.time)),(0,E.jsx)(`td`,{children:(0,E.jsx)(`strong`,{children:z(n.gradientTotals.recurrentWeight)})})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`dL/db`}),o.map(e=>(0,E.jsx)(`td`,{children:z(e.parameterContributions.bias)},e.time)),(0,E.jsx)(`td`,{children:(0,E.jsx)(`strong`,{children:z(n.gradientTotals.bias)})})]})]})]})}),(0,E.jsxs)(`p`,{className:`bptt-initial-gradient`,children:[`The reverse chain continues into the explicit initial state:`,(0,E.jsxs)(`strong`,{children:[` dL/dh[-1] = `,z(n.gradientTotals.initialState)]})]})]}),(0,E.jsxs)(`section`,{className:`bptt-audit-grid`,"aria-label":`Gradient audit and update preview`,children:[(0,E.jsxs)(`div`,{className:`bptt-panel`,children:[(0,E.jsxs)(`div`,{className:`bptt-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Independent oracle`}),(0,E.jsx)(`h2`,{children:`Finite-difference gradient check`})]}),(0,E.jsx)(`strong`,{className:`bptt-pass`,children:`PASS`})]}),(0,E.jsx)(`div`,{className:`bptt-table-wrap`,children:(0,E.jsxs)(`table`,{className:`bptt-table`,children:[(0,E.jsx)(`caption`,{children:`Analytical and numerical gradient agreement`}),(0,E.jsx)(`thead`,{children:(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`col`,children:`parameter`}),(0,E.jsx)(`th`,{scope:`col`,children:`BPTT`}),(0,E.jsx)(`th`,{scope:`col`,children:`numerical`}),(0,E.jsx)(`th`,{scope:`col`,children:`error`})]})}),(0,E.jsx)(`tbody`,{children:[`inputWeight`,`recurrentWeight`,`bias`].map(e=>(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:e===`inputWeight`?`W_x`:e===`recurrentWeight`?`W_h`:`b`}),(0,E.jsx)(`td`,{children:z(n.gradientTotals[e])}),(0,E.jsx)(`td`,{children:z(n.numericalGradients[e])}),(0,E.jsx)(`td`,{children:z(n.gradientErrors[e])})]},e))})]})})]}),(0,E.jsxs)(`div`,{className:`bptt-panel bptt-update-panel`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One step · learning rate 0.1`}),(0,E.jsx)(`h2`,{children:`Move against the accumulated gradient`}),(0,E.jsxs)(`div`,{className:`bptt-loss-change`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`before loss`}),(0,E.jsx)(`strong`,{children:z(n.loss)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`after loss`}),(0,E.jsx)(`strong`,{children:z(n.update.loss)})]})]}),(0,E.jsxs)(`div`,{className:`bptt-parameter-update`,children:[(0,E.jsxs)(`code`,{children:[`W_x: `,z(yl.inputWeight),` → `,z(n.update.parameters.inputWeight)]}),(0,E.jsxs)(`code`,{children:[`W_h: `,z(yl.recurrentWeight),` → `,z(n.update.parameters.recurrentWeight)]}),(0,E.jsxs)(`code`,{children:[`b: `,z(yl.bias),` → `,z(n.update.parameters.bias)]})]}),(0,E.jsxs)(`p`,{children:[`Updated states = [`,n.update.states.map(z).join(`, `),`]`]})]})]})]}),(0,E.jsxs)(`aside`,{className:`recurrent-controls bptt-controls`,"aria-label":`BPTT microscope controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Forward and backward belong together`}),(0,E.jsx)(`h2`,{children:`Reverse the unroll`}),(0,E.jsx)(`p`,{children:`Select a reverse-time cell. Its future gradient was produced by the cell immediately to its right in the forward graph.`}),(0,E.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:e,children:`Show forward unroll`}),(0,E.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:t,children:`Compare GRU and LSTM gates`}),(0,E.jsxs)(`div`,{className:`recurrent-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected reverse step`}),(0,E.jsxs)(`strong`,{children:[`t = `,r]}),(0,E.jsxs)(`span`,{children:[z(a.directStateGradient),` direct + `,z(a.futureStateGradient),` from the future.`]})]}),(0,E.jsxs)(`div`,{className:`recurrent-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Vectors use the same reverse walk with matrix products. GRUs and LSTMs add gates, while truncated BPTT limits how far this lane runs.`})]})]})]})}var Ol=.8,kl=.8,Al=Math.log(3),jl=Math.atanh(.6),Ml=jl-.4;function Nl(e){if(e>=0)return 1/(1+Math.exp(-e));let t=Math.exp(e);return t/(1+t)}function Pl(e){return{preactivation:e,value:Nl(e)}}function Fl(e=1,t=Ol,n=kl){if(![e,t,n].every(Number.isFinite))throw Error(`NN11 V1 needs finite scalar input and recurrent states.`);let r=Pl(0),i=Pl(-Al),a=r.value*t,o=0*e,s=a,c=o+s+Ml,l=Math.tanh(c),u=(1-i.value)*t,d=i.value*l,f=Pl(0),p=Pl(-Al),m=Pl(Al),h={preactivation:jl,value:Math.tanh(jl)},g=f.value*n,_=p.value*h.value,v=g+_,y=Math.tanh(v);return{input:e,previousHidden:t,previousCell:n,gru:{resetGate:r,updateGate:i,candidate:{inputProduct:o,resetState:a,recurrentProduct:s,bias:Ml,preactivation:c,value:l},retainedState:u,candidateWrite:d,hiddenState:u+d},lstm:{forgetGate:f,inputGate:p,outputGate:m,candidate:h,retainedCell:g,candidateWrite:_,cellState:v,exposedCell:y,hiddenState:m.value*y}}}function Il(e,t,n,r=Fl()){if(!Number.isFinite(n)||n<0||n>1)throw Error(`NN11 gate interventions must be between zero and one.`);if(e===`gru`){if(t!==`reset`&&t!==`update`)throw Error(`Gate ${t} does not belong to the GRU.`);let i=t===`reset`?n:r.gru.resetGate.value,a=t===`update`?n:r.gru.updateGate.value,o=Math.tanh(r.gru.candidate.inputProduct+i*r.previousHidden+r.gru.candidate.bias);return{model:e,gate:t,gateValue:n,candidate:o,cellState:null,hiddenState:(1-a)*r.previousHidden+a*o}}if(![`forget`,`input`,`output`].includes(t))throw Error(`Gate ${t} does not belong to the LSTM.`);let i=t===`forget`?n:r.lstm.forgetGate.value,a=t===`input`?n:r.lstm.inputGate.value,o=t===`output`?n:r.lstm.outputGate.value,s=i*r.previousCell+a*r.lstm.candidate.value;return{model:e,gate:t,gateValue:n,candidate:r.lstm.candidate.value,cellState:s,hiddenState:o*Math.tanh(s)}}function Ll(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(6)).toString()}function Rl(e,t){return e===`gru`?t===`reset`?.5:.25:t===`forget`?.5:t===`input`?.25:.75}function zl({onShowBackward:e}){let t=(0,l.useMemo)(()=>Fl(),[]),[n,r]=(0,l.useState)(`gru`),[i,a]=(0,l.useState)(`update`),[o,s]=(0,l.useState)(`canonical`),c=o===`canonical`?Rl(n,i):o,u=Il(n,i,c,t),d=(e,t)=>{r(e),a(t),s(`canonical`)},f=n===`gru`&&i===`reset`?c:t.gru.resetGate.value,p=n===`gru`&&i===`update`?c:t.gru.updateGate.value,m=n===`gru`?u.candidate:t.gru.candidate.value,h=(1-p)*t.previousHidden,g=p*m,_=h+g,v=n===`lstm`&&i===`forget`?c:t.lstm.forgetGate.value,y=n===`lstm`&&i===`input`?c:t.lstm.inputGate.value,b=n===`lstm`&&i===`output`?c:t.lstm.outputGate.value,x=v*t.previousCell,S=y*t.lstm.candidate.value,C=x+S,w=b*Math.tanh(C);return(0,E.jsxs)(`main`,{className:`workspace workspace--gates`,children:[(0,E.jsxs)(`section`,{className:`gate-stage`,"aria-label":`GRU and LSTM gate comparison`,children:[(0,E.jsxs)(`div`,{className:`gate-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN11 · gated sequence memory`}),(0,E.jsx)(`h2`,{children:`GRU and LSTM gate comparator`}),(0,E.jsx)(`p`,{children:`Route the same previous memory and candidate through both cells. Change one gate while every other signal stays fixed.`})]}),(0,E.jsxs)(`div`,{className:`gate-input-chip`,children:[(0,E.jsx)(`small`,{children:`shared input`}),(0,E.jsx)(`strong`,{children:`x = 1 · h = 0.8`})]})]}),(0,E.jsxs)(`section`,{className:`gate-comparison-panel`,"aria-label":`Aligned gated memory lanes`,children:[(0,E.jsxs)(`div`,{className:`gate-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same evidence · different state design`}),(0,E.jsx)(`h2`,{children:`Follow what each gate lets through`})]}),(0,E.jsx)(`code`,{children:`candidate = 0.6`})]}),(0,E.jsxs)(`article`,{className:`gate-model-lane gate-model-lane--gru`,"aria-label":`GRU memory lane`,children:[(0,E.jsxs)(`div`,{className:`gate-model-label`,children:[(0,E.jsx)(`span`,{children:`GRU`}),(0,E.jsx)(`strong`,{children:`one stored and exposed state`})]}),(0,E.jsxs)(`div`,{className:`gate-flow`,children:[(0,E.jsxs)(`div`,{className:`gate-state-node`,children:[(0,E.jsx)(`small`,{children:`previous state`}),(0,E.jsxs)(`strong`,{children:[`h = `,Ll(t.previousHidden)]})]}),(0,E.jsxs)(`button`,{"aria-label":`Select GRU reset gate`,"aria-pressed":n===`gru`&&i===`reset`,className:n===`gru`&&i===`reset`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`gru`,`reset`),children:[(0,E.jsx)(`small`,{children:`reset r`}),(0,E.jsx)(`strong`,{children:Ll(f)}),(0,E.jsxs)(`span`,{children:[`candidate sees `,Ll(f*t.previousHidden)]})]}),(0,E.jsxs)(`div`,{className:`gate-candidate-node`,children:[(0,E.jsx)(`small`,{children:`candidate n`}),(0,E.jsx)(`strong`,{children:Ll(m)})]}),(0,E.jsxs)(`button`,{"aria-label":`Select GRU update gate`,"aria-pressed":n===`gru`&&i===`update`,className:n===`gru`&&i===`update`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`gru`,`update`),children:[(0,E.jsx)(`small`,{children:`update z`}),(0,E.jsx)(`strong`,{children:Ll(p)}),(0,E.jsx)(`span`,{children:`new share`})]}),(0,E.jsxs)(`div`,{className:`gate-result-node`,children:[(0,E.jsx)(`small`,{children:`next hidden`}),(0,E.jsxs)(`strong`,{children:[`h = `,Ll(_)]}),(0,E.jsxs)(`span`,{children:[Ll(h),` old + `,Ll(g),` new`]})]})]})]}),(0,E.jsxs)(`article`,{className:`gate-model-lane gate-model-lane--lstm`,"aria-label":`LSTM memory lane`,children:[(0,E.jsxs)(`div`,{className:`gate-model-label`,children:[(0,E.jsx)(`span`,{children:`LSTM`}),(0,E.jsx)(`strong`,{children:`private cell plus exposed hidden state`})]}),(0,E.jsxs)(`div`,{className:`gate-flow gate-flow--lstm`,children:[(0,E.jsxs)(`div`,{className:`gate-state-node`,children:[(0,E.jsx)(`small`,{children:`previous cell`}),(0,E.jsxs)(`strong`,{children:[`c = `,Ll(t.previousCell)]})]}),(0,E.jsxs)(`button`,{"aria-label":`Select LSTM forget gate`,"aria-pressed":n===`lstm`&&i===`forget`,className:n===`lstm`&&i===`forget`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`lstm`,`forget`),children:[(0,E.jsx)(`small`,{children:`forget f`}),(0,E.jsx)(`strong`,{children:Ll(v)}),(0,E.jsx)(`span`,{children:`old share`})]}),(0,E.jsxs)(`button`,{"aria-label":`Select LSTM input gate`,"aria-pressed":n===`lstm`&&i===`input`,className:n===`lstm`&&i===`input`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`lstm`,`input`),children:[(0,E.jsx)(`small`,{children:`input i`}),(0,E.jsx)(`strong`,{children:Ll(y)}),(0,E.jsx)(`span`,{children:`candidate share`})]}),(0,E.jsxs)(`div`,{className:`gate-cell-node`,children:[(0,E.jsx)(`small`,{children:`private cell`}),(0,E.jsxs)(`strong`,{children:[`c = `,Ll(C)]}),(0,E.jsxs)(`span`,{children:[Ll(x),` old + `,Ll(S),` new`]})]}),(0,E.jsxs)(`button`,{"aria-label":`Select LSTM output gate`,"aria-pressed":n===`lstm`&&i===`output`,className:n===`lstm`&&i===`output`?`gate-node gate-node--active`:`gate-node`,type:`button`,onClick:()=>d(`lstm`,`output`),children:[(0,E.jsx)(`small`,{children:`output o`}),(0,E.jsx)(`strong`,{children:Ll(b)}),(0,E.jsx)(`span`,{children:`visible share`})]}),(0,E.jsxs)(`div`,{className:`gate-result-node`,children:[(0,E.jsx)(`small`,{children:`next hidden`}),(0,E.jsxs)(`strong`,{children:[`h = `,Ll(w)]}),(0,E.jsx)(`span`,{children:`o × tanh(c)`})]})]})]})]}),(0,E.jsxs)(`section`,{className:`gate-comparison-panel`,"aria-label":`Gate responsibility comparison`,children:[(0,E.jsx)(`div`,{className:`gate-panel-heading`,children:(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Architecture, not acronym memorization`}),(0,E.jsx)(`h2`,{children:`Which signal does each gate control?`})]})}),(0,E.jsx)(`div`,{className:`gate-table-wrap`,children:(0,E.jsxs)(`table`,{className:`gate-table`,children:[(0,E.jsx)(`caption`,{children:`GRU and LSTM state-routing responsibilities`}),(0,E.jsx)(`thead`,{children:(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`col`,children:`Responsibility`}),(0,E.jsx)(`th`,{scope:`col`,children:`GRU`}),(0,E.jsx)(`th`,{scope:`col`,children:`LSTM`})]})}),(0,E.jsxs)(`tbody`,{children:[(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`Build candidate`}),(0,E.jsx)(`td`,{children:`reset gate`}),(0,E.jsx)(`td`,{children:`candidate tanh path`})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`Retain old memory`}),(0,E.jsx)(`td`,{rowSpan:2,children:`update gate mixes both`}),(0,E.jsx)(`td`,{children:`forget gate`})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`Write new memory`}),(0,E.jsx)(`td`,{children:`input gate`})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`Expose memory`}),(0,E.jsx)(`td`,{children:`same hidden state`}),(0,E.jsx)(`td`,{children:`output gate`})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`State buffers`}),(0,E.jsxs)(`td`,{children:[`h = `,Ll(_)]}),(0,E.jsxs)(`td`,{children:[`c = `,Ll(C),`, h = `,Ll(w)]})]})]})]})})]})]}),(0,E.jsxs)(`aside`,{className:`gate-controls`,"aria-label":`Gate intervention controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One controlled intervention`}),(0,E.jsxs)(`h2`,{children:[n.toUpperCase(),` `,i,` gate`]}),(0,E.jsx)(`p`,{children:`Keep every other gate fixed. Use the learned canonical value or force this one valve fully closed or open.`}),(0,E.jsxs)(`div`,{className:`gate-intervention-buttons`,"aria-label":`Selected gate value`,children:[(0,E.jsx)(`button`,{"aria-pressed":o===`canonical`,type:`button`,onClick:()=>s(`canonical`),children:`Canonical`}),(0,E.jsx)(`button`,{"aria-pressed":o===0,type:`button`,onClick:()=>s(0),children:`Force 0`}),(0,E.jsx)(`button`,{"aria-pressed":o===1,type:`button`,onClick:()=>s(1),children:`Force 1`})]}),(0,E.jsxs)(`div`,{className:`gate-selected-summary`,"aria-label":`Selected gate effect`,children:[(0,E.jsx)(`small`,{children:`selected value`}),(0,E.jsx)(`strong`,{children:Ll(c)}),(0,E.jsx)(`span`,{children:n===`gru`?`candidate ${Ll(m)} · next h ${Ll(_)}`:`next c ${Ll(C)} · visible h ${Ll(w)}`})]}),(0,E.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:e,children:`Return to BPTT gradients`}),(0,E.jsxs)(`div`,{className:`recurrent-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Vector cells pack each gate's affine projection into matrices. The scalar routing stays identical at every coordinate.`})]})]})]})}function Bl(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function Vl({onShowBackward:e}){let[t,n]=(0,l.useState)(0),[r,i]=(0,l.useState)(!0),a=(0,l.useMemo)(()=>Cl(),[]),o=(0,l.useMemo)(()=>Cl(vl,0,yl,!1),[]),s=r?a:o,c=s.steps[t];return(0,E.jsxs)(`main`,{className:`workspace workspace--recurrent`,children:[(0,E.jsxs)(`section`,{className:`recurrent-stage`,"aria-label":`Three-step recurrent state trace`,children:[(0,E.jsxs)(`div`,{className:`recurrent-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN09 · sequence networks`}),(0,E.jsx)(`h2`,{children:`Recurrent-state unroller`}),(0,E.jsx)(`p`,{children:`Run one scalar cell three times. Each result becomes part of the next input while one parameter set stays shared across time.`})]}),(0,E.jsx)(`div`,{className:`recurrent-sequence-chip`,children:`x = [1, 2, 0]`})]}),(0,E.jsxs)(`section`,{className:`recurrent-unroll-panel`,"aria-label":`Recurrent cell unroll`,children:[(0,E.jsxs)(`div`,{className:`recurrent-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One cell · three executions`}),(0,E.jsx)(`h2`,{children:`Follow the state from left to right`})]}),(0,E.jsxs)(`div`,{className:`recurrent-final-state`,children:[(0,E.jsx)(`small`,{children:`final state`}),(0,E.jsx)(`strong`,{children:Bl(s.finalState)})]})]}),(0,E.jsxs)(`div`,{className:`shared-parameter-strip`,"aria-label":`Parameters shared by every time step`,children:[(0,E.jsx)(`span`,{children:`shared at t=0, 1, 2`}),(0,E.jsxs)(`code`,{children:[`Wₓ = `,Bl(yl.inputWeight)]}),(0,E.jsxs)(`code`,{children:[`Wₕ = `,Bl(yl.recurrentWeight)]}),(0,E.jsxs)(`code`,{children:[`b = `,Bl(yl.bias)]})]}),(0,E.jsxs)(`div`,{className:`recurrent-chain`,"aria-label":`Unrolled recurrent state chain`,children:[(0,E.jsxs)(`div`,{className:`recurrent-initial-node`,children:[(0,E.jsx)(`small`,{children:`initial`}),(0,E.jsx)(`strong`,{children:`h[-1]`}),(0,E.jsx)(`code`,{children:Bl(0)})]}),s.steps.map(e=>(0,E.jsxs)(l.Fragment,{children:[(0,E.jsxs)(`div`,{className:r?`recurrent-connector`:`recurrent-connector recurrent-connector--disabled`,children:[(0,E.jsx)(`small`,{children:r?`carry h`:`cut`}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`→`})]}),(0,E.jsxs)(`button`,{"aria-label":`Select recurrent step ${e.time}`,"aria-pressed":t===e.time,className:t===e.time?`recurrent-cell recurrent-cell--active`:`recurrent-cell`,type:`button`,onClick:()=>n(e.time),children:[(0,E.jsxs)(`small`,{children:[`time `,e.time]}),(0,E.jsxs)(`span`,{children:[`x[`,e.time,`] = `,Bl(e.input)]}),(0,E.jsxs)(`strong`,{children:[`h[`,e.time,`] = `,Bl(e.state)]})]})]},e.time))]}),(0,E.jsxs)(`div`,{className:`recurrent-arithmetic`,"aria-label":`Selected recurrent arithmetic`,children:[(0,E.jsxs)(`div`,{className:`recurrent-arithmetic-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected · time `,t]}),(0,E.jsx)(`h3`,{children:`Open this cell`})]}),(0,E.jsxs)(`code`,{children:[`h[`,t-1,`] → h[`,t,`]`]})]}),(0,E.jsxs)(`div`,{className:`recurrent-equation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`new input`}),(0,E.jsxs)(`strong`,{children:[`2 × `,Bl(c.input),` = `,Bl(c.inputProduct)]})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{className:r?``:`equation-term--disabled`,children:[(0,E.jsx)(`small`,{children:`carried state`}),(0,E.jsxs)(`strong`,{children:[`0.5 × `,Bl(c.previousState),` = `,Bl(c.recurrentProduct)]})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias`}),(0,E.jsx)(`strong`,{children:Bl(c.bias)})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`preactivation`}),(0,E.jsx)(`strong`,{children:Bl(c.preactivation)})]}),(0,E.jsx)(`span`,{children:`→`}),(0,E.jsxs)(`div`,{className:`recurrent-equation__state`,children:[(0,E.jsx)(`small`,{children:`ReLU state`}),(0,E.jsx)(`strong`,{children:Bl(c.state)})]})]})]})]}),(0,E.jsxs)(`section`,{className:`memory-ablation-panel`,"aria-label":`Recurrent memory ablation`,children:[(0,E.jsxs)(`div`,{className:`recurrent-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same inputs · memory removed`}),(0,E.jsx)(`h2`,{children:`What came through the recurrent link?`})]}),(0,E.jsx)(`p`,{children:`The final zero input remembers earlier steps only when the link is present.`})]}),(0,E.jsx)(`div`,{className:`recurrent-table-wrap`,children:(0,E.jsxs)(`table`,{className:`recurrent-table`,children:[(0,E.jsx)(`caption`,{children:`State comparison with and without recurrence`}),(0,E.jsx)(`thead`,{children:(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`col`,children:`time`}),(0,E.jsx)(`th`,{scope:`col`,children:`input`}),(0,E.jsx)(`th`,{scope:`col`,children:`with memory`}),(0,E.jsx)(`th`,{scope:`col`,children:`without memory`}),(0,E.jsx)(`th`,{scope:`col`,children:`difference`})]})}),(0,E.jsx)(`tbody`,{children:a.steps.map((e,n)=>{let r=o.states[n];return(0,E.jsxs)(`tr`,{className:t===n?`recurrent-table-row--active`:``,children:[(0,E.jsx)(`th`,{scope:`row`,children:n}),(0,E.jsx)(`td`,{children:Bl(e.input)}),(0,E.jsx)(`td`,{children:Bl(e.state)}),(0,E.jsx)(`td`,{children:Bl(r)}),(0,E.jsx)(`td`,{children:Bl(e.state-r)})]},n)})})]})})]})]}),(0,E.jsxs)(`aside`,{className:`recurrent-controls`,"aria-label":`Recurrent unroll controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One honest experiment`}),(0,E.jsx)(`h2`,{children:`Memory control`}),(0,E.jsx)(`p`,{children:`Select a time-step cell, then cut the recurrent link without changing its inputs, weights, or bias.`}),(0,E.jsx)(`button`,{className:`bptt-view-button`,type:`button`,onClick:e,children:`Trace backward gradients`}),(0,E.jsxs)(`label`,{className:`recurrent-memory-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:r,onChange:e=>i(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Carry the previous state`}),(0,E.jsx)(`small`,{children:`Use Wₕ × h[t - 1] at every step.`})]})]}),(0,E.jsxs)(`div`,{className:`recurrent-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected time`}),(0,E.jsxs)(`strong`,{children:[`t = `,t]}),(0,E.jsx)(`span`,{children:r?`${Bl(c.recurrentProduct)} enters through memory.`:`The recurrent contribution is forced to zero.`})]}),(0,E.jsxs)(`div`,{className:`recurrent-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`Vector states repeat this same pattern across several coordinates. Backpropagation will reverse the unrolled arrows and add gradient contributions into the shared parameters.`})]})]})]})}function Hl(){let[e,t]=(0,l.useState)(`forward`);return e===`backward`?(0,E.jsx)(Dl,{onShowForward:()=>t(`forward`),onShowGates:()=>t(`gates`)}):e===`gates`?(0,E.jsx)(zl,{onShowBackward:()=>t(`backward`)}):(0,E.jsx)(Vl,{onShowBackward:()=>t(`backward`)})}var Ul=[2,-1],Wl={encoder:{weights:[.5,-.25],bias:0},decoder:{weights:[1.2,-.8],bias:[.1,-.2]}},B=.1,Gl=[`encoder.weights[0]`,`encoder.weights[1]`,`encoder.bias`,`decoder.weights[0]`,`decoder.weights[1]`,`decoder.bias[0]`,`decoder.bias[1]`];function V(e){return Math.abs(e)<1e-12?0:e}function H(e,t){return e.length===t&&e.every(Number.isFinite)}function Kl(e){return{encoder:{weights:[...e.encoder.weights],bias:e.encoder.bias},decoder:{weights:[...e.decoder.weights],bias:[...e.decoder.bias]}}}function ql(e,t){let n=e.map((e,n)=>V(e*t.encoder.weights[n])),r=V(n.reduce((e,t)=>e+t,0)+t.encoder.bias),i=t.decoder.weights.map(e=>V(r*e)),a=i.map((e,n)=>V(e+t.decoder.bias[n])),o=a.map((t,n)=>V(t-e[n])),s=o.map(e=>e*e);return{encoderProducts:n,bottleneck:r,decoderProducts:i,reconstruction:a,errors:o,squaredErrors:s,loss:s.reduce((e,t)=>e+t,0)/2}}function Jl(e){return[...e.encoder.weights,e.encoder.bias,...e.decoder.weights,...e.decoder.bias]}function Yl(e){return{encoder:{weights:e.slice(0,2),bias:e[2]},decoder:{weights:e.slice(3,5),bias:e.slice(5,7)}}}function Xl(e=B,t=Ul,n=Wl){if(!Number.isFinite(e)||e<=0||!H(t,2)||!H(n.encoder.weights,2)||!Number.isFinite(n.encoder.bias)||!H(n.decoder.weights,2)||!H(n.decoder.bias,2))throw Error(`NN16 V1 needs a two-number input, 2 -> 1 -> 2 finite parameters, and a positive learning rate.`);let r=Kl(n),i=ql(t,r),a=[...i.errors],o=a.map(e=>V(e*i.bottleneck)),s=[...a],c=a.map((e,t)=>V(e*r.decoder.weights[t])),l=V(c.reduce((e,t)=>e+t,0)),u=t.map(e=>V(l*e)),d=l,f={reconstructionGradients:a,decoderWeightGradients:o,decoderBiasGradients:s,bottleneckGradientContributions:c,bottleneckGradient:l,encoderWeightGradients:u,encoderBiasGradient:d},p=[...u,d,...o,...s],m=Jl(r),h=1e-6,g=m.map((e,n)=>{let r=[...m],i=[...m];return r[n]+=h,i[n]-=h,(ql(t,Yl(r)).loss-ql(t,Yl(i)).loss)/(2*h)}),_=Math.max(...p.map((e,t)=>Math.abs(e-g[t]))),v={encoder:{weights:r.encoder.weights.map((t,n)=>t-e*u[n]),bias:r.encoder.bias-e*d},decoder:{weights:r.decoder.weights.map((t,n)=>t-e*o[n]),bias:r.decoder.bias.map((t,n)=>t-e*s[n])}},y=ql(t,v);return{input:[...t],learningRate:e,parameters:r,forward:i,backward:f,gradientCheck:{epsilon:h,parameterOrder:[...Gl],analytical:p,numerical:g,maxAbsoluteError:_},updatedParameters:v,postUpdate:y}}function U(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function W(e){return`[${e.map(U).join(`, `)}]`}function Zl(){let e=(0,l.useMemo)(()=>Xl(),[]),[t,n]=(0,l.useState)(0),[r,i]=(0,l.useState)(!1),a=r?e.postUpdate:e.forward,o=r?e.updatedParameters:e.parameters;return(0,E.jsxs)(`main`,{className:`workspace workspace--autoencoder`,children:[(0,E.jsxs)(`section`,{className:`autoencoder-stage`,"aria-label":`Two-number autoencoder bottleneck trace`,children:[(0,E.jsxs)(`div`,{className:`autoencoder-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN16 - representation through constraint`}),(0,E.jsx)(`h2`,{children:`Two numbers through one bottleneck`}),(0,E.jsx)(`p`,{children:`Compress a two-coordinate input into one scalar, reconstruct both coordinates from that shared value, and follow both errors back through one audited SGD step.`})]}),(0,E.jsx)(`div`,{className:`autoencoder-chip`,children:`2 -> 1 -> 2`})]}),(0,E.jsxs)(`section`,{className:`autoencoder-network-panel`,"aria-label":`Autoencoder encode and decode path`,children:[(0,E.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`The decoder never sees the original pair`}),(0,E.jsx)(`h2`,{children:`One scalar must serve two reconstructions`})]}),(0,E.jsx)(`code`,{children:r?`after one SGD step`:`saved forward pass`})]}),(0,E.jsxs)(`div`,{className:`autoencoder-network`,children:[(0,E.jsxs)(`div`,{className:`autoencoder-input-stack`,children:[(0,E.jsx)(`small`,{children:`input is also target`}),e.input.map((e,t)=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`span`,{children:[`x`,t]}),(0,E.jsx)(`strong`,{children:U(e)})]},t))]}),(0,E.jsx)(`span`,{className:`autoencoder-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`autoencoder-encoder-stack`,children:[(0,E.jsx)(`small`,{children:`encoder products`}),a.encoderProducts.map((t,n)=>(0,E.jsxs)(`code`,{children:[U(e.input[n]),` x `,U(o.encoder.weights[n]),` = `,U(t)]},n)),(0,E.jsxs)(`code`,{children:[`+ bias `,U(o.encoder.bias)]})]}),(0,E.jsx)(`span`,{className:`autoencoder-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`autoencoder-bottleneck`,children:[(0,E.jsx)(`small`,{children:`bottleneck z`}),(0,E.jsx)(`strong`,{children:U(a.bottleneck)}),(0,E.jsx)(`span`,{children:`one saved number`})]}),(0,E.jsx)(`span`,{className:`autoencoder-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`autoencoder-output-stack`,children:[(0,E.jsx)(`small`,{children:`decoder reconstructions`}),a.reconstruction.map((r,i)=>(0,E.jsxs)(`button`,{"aria-label":`Select reconstruction ${i}`,"aria-pressed":t===i,type:`button`,onClick:()=>n(i),children:[(0,E.jsxs)(`span`,{children:[`x_hat`,i]}),(0,E.jsx)(`strong`,{children:U(r)}),(0,E.jsxs)(`small`,{children:[`target `,U(e.input[i])]})]},i))]})]})]}),(0,E.jsxs)(`section`,{className:`autoencoder-reconstruction-panel`,"aria-label":`Selected autoencoder reconstruction ${t}`,children:[(0,E.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected - reconstruction `,t]}),(0,E.jsx)(`h2`,{children:`Decode and measure one coordinate`})]}),(0,E.jsxs)(`div`,{className:`autoencoder-loss-badge`,children:[(0,E.jsx)(`small`,{children:`total mean loss`}),(0,E.jsx)(`strong`,{children:U(a.loss)})]})]}),(0,E.jsxs)(`div`,{className:`autoencoder-reconstruction-flow`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`shared bottleneck`}),(0,E.jsxs)(`code`,{children:[`z = `,U(a.bottleneck)]})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`x`}),(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`decoder weight `,t]}),(0,E.jsx)(`code`,{children:U(o.decoder.weights[t])})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`decoder bias `,t]}),(0,E.jsx)(`code`,{children:U(o.decoder.bias[t])})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,E.jsxs)(`div`,{className:`autoencoder-reconstruction-result`,children:[(0,E.jsx)(`small`,{children:`reconstruction`}),(0,E.jsx)(`strong`,{children:U(a.reconstruction[t])})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`-`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`input target`}),(0,E.jsx)(`code`,{children:U(e.input[t])})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,E.jsxs)(`div`,{className:`autoencoder-error-result`,children:[(0,E.jsx)(`small`,{children:`error / loss gradient`}),(0,E.jsx)(`strong`,{children:U(a.errors[t])}),(0,E.jsxs)(`code`,{children:[`squared `,U(a.squaredErrors[t])]})]})]})]}),(0,E.jsxs)(`section`,{className:`autoencoder-backward-panel`,"aria-label":`Autoencoder bottleneck gradient trace`,children:[(0,E.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Two decoder branches meet at z`}),(0,E.jsx)(`h2`,{children:`Reconstruction error flows back through compression`})]}),(0,E.jsx)(`code`,{children:`dL/dz = sum of both routes`})]}),(0,E.jsxs)(`div`,{className:`autoencoder-branch-gradients`,children:[e.backward.bottleneckGradientContributions.map((r,i)=>(0,E.jsxs)(`button`,{"aria-label":`Select reconstruction gradient ${i}`,"aria-pressed":t===i,type:`button`,onClick:()=>n(i),children:[(0,E.jsxs)(`small`,{children:[`output `,i,` route`]}),(0,E.jsxs)(`code`,{children:[U(e.backward.reconstructionGradients[i]),` x `,U(e.parameters.decoder.weights[i])]}),(0,E.jsx)(`strong`,{children:U(r)})]},i)),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`sum`}),(0,E.jsxs)(`div`,{className:`autoencoder-bottleneck-gradient`,children:[(0,E.jsx)(`small`,{children:`bottleneck gradient`}),(0,E.jsx)(`strong`,{children:U(e.backward.bottleneckGradient)})]})]}),(0,E.jsxs)(`div`,{className:`autoencoder-gradient-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder weight gradients`}),(0,E.jsx)(`code`,{children:W(e.backward.decoderWeightGradients)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder bias gradients`}),(0,E.jsx)(`code`,{children:W(e.backward.decoderBiasGradients)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`encoder weight gradients`}),(0,E.jsx)(`code`,{children:W(e.backward.encoderWeightGradients)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`encoder bias gradient`}),(0,E.jsx)(`code`,{children:U(e.backward.encoderBiasGradient)})]})]})]}),(0,E.jsxs)(`section`,{className:`autoencoder-update-panel`,"aria-label":`Autoencoder SGD update and gradient audit`,children:[(0,E.jsxs)(`div`,{className:`autoencoder-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`All seven parameters move together`}),(0,E.jsx)(`h2`,{children:`Audit, update, rerun`})]}),(0,E.jsxs)(`code`,{children:[`parameter - `,e.learningRate,` x gradient`]})]}),(0,E.jsxs)(`div`,{className:`autoencoder-parameter-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`encoder before`}),(0,E.jsxs)(`code`,{children:[`w `,W(e.parameters.encoder.weights)]}),(0,E.jsxs)(`code`,{children:[`b `,U(e.parameters.encoder.bias)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`encoder after`}),(0,E.jsxs)(`code`,{children:[`w `,W(e.updatedParameters.encoder.weights)]}),(0,E.jsxs)(`code`,{children:[`b `,U(e.updatedParameters.encoder.bias)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder before`}),(0,E.jsxs)(`code`,{children:[`w `,W(e.parameters.decoder.weights)]}),(0,E.jsxs)(`code`,{children:[`b `,W(e.parameters.decoder.bias)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder after`}),(0,E.jsxs)(`code`,{children:[`w `,W(e.updatedParameters.decoder.weights)]}),(0,E.jsxs)(`code`,{children:[`b `,W(e.updatedParameters.decoder.bias)]})]})]}),(0,E.jsxs)(`div`,{className:`autoencoder-gradient-audit`,children:[(0,E.jsx)(`span`,{children:`Central finite differences - 7 parameters`}),(0,E.jsxs)(`code`,{children:[`epsilon = `,e.gradientCheck.epsilon]}),(0,E.jsxs)(`strong`,{children:[`max error `,e.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,E.jsxs)(`div`,{className:`autoencoder-loss-drop`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`loss before`}),(0,E.jsx)(`strong`,{children:U(e.forward.loss)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`loss after`}),(0,E.jsx)(`strong`,{children:U(e.postUpdate.loss)})]}),(0,E.jsx)(`p`,{children:`One reconstruction improves sharply; the shared mean objective falls.`})]})]})]}),(0,E.jsxs)(`aside`,{className:`autoencoder-controls`,"aria-label":`Autoencoder trace controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Open one decoder branch`}),(0,E.jsx)(`h2`,{children:`Bottleneck controls`}),(0,E.jsx)(`p`,{children:`Both outputs stay visible. Selection follows one reconstruction's arithmetic and gradient route without disconnecting the shared scalar.`}),(0,E.jsx)(`div`,{className:`attention-query-buttons`,"aria-label":`Autoencoder reconstruction selection`,children:[0,1].map(e=>(0,E.jsxs)(`button`,{"aria-pressed":t===e,type:`button`,onClick:()=>n(e),children:[`output `,e]},e))}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:r,onChange:e=>i(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Use updated parameters`}),(0,E.jsx)(`small`,{children:`Rerun encode, decode, and loss after one SGD step.`})]})]}),(0,E.jsxs)(`div`,{className:`attention-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected reconstruction`}),(0,E.jsxs)(`strong`,{children:[`x_hat`,t]}),(0,E.jsxs)(`span`,{children:[U(a.reconstruction[t]),` versus target `,U(e.input[t])]})]}),(0,E.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,E.jsx)(`span`,{children:`What is actually compressed?`}),(0,E.jsx)(`p`,{children:`The decoder receives z only. It cannot inspect either original coordinate while rebuilding the pair.`})]}),(0,E.jsxs)(`div`,{className:`attention-next-note`,children:[(0,E.jsx)(`span`,{children:`Keep the claim small`}),(0,E.jsx)(`p`,{children:`One example explains the mechanics. A useful representation needs many examples to reveal a shared lower-dimensional pattern.`})]})]})]})}var Ql=-.5,$l=[{t:1,beta:.36,normalizedT:.5},{t:2,beta:.4375,normalizedT:1}],eu={sampleWeight:0,timestepWeight:0,bias:0},tu=.5,nu=[`denoiser.sample_weight`,`denoiser.timestep_weight`,`denoiser.bias`];function ru(e){return{...e}}function iu(e,t,n){let r=1;return n.map(n=>{let i=1-n.beta;r*=i;let a=Math.sqrt(r),o=Math.sqrt(1-r),s=a*e,c=o*t;return{...n,alpha:i,alphaBar:r,signalScale:a,noiseScale:o,signalContribution:s,noiseContribution:c,noisySample:s+c}})}function au(e,t,n){let r=e.map(e=>{let r=n.sampleWeight*e.noisySample+n.timestepWeight*e.normalizedT+n.bias,i=r-t;return{t:e.t,noisySample:e.noisySample,normalizedT:e.normalizedT,predictedNoise:r,targetNoise:t,error:i,loss:.5*i*i}});return{rows:r,meanLoss:r.reduce((e,t)=>e+t.loss,0)/r.length}}function ou(e,t){let n=e[e.length-1].noisySample;return[...e].reverse().map(e=>{let r=t.sampleWeight*n+t.timestepWeight*e.normalizedT+t.bias,i=e.beta/e.noiseScale,a=i*r,o=n-a,s=Math.sqrt(e.alpha),c=o/s,l={t:e.t,inputSample:n,normalizedT:e.normalizedT,predictedNoise:r,noiseCoefficient:i,scaledNoiseCorrection:a,correctedSample:o,alphaScale:s,outputMean:c};return n=c,l})}function su(e=1,t=Ql,n=tu,r=eu,i=$l){if(![e,t,n,r.sampleWeight,r.timestepWeight,r.bias,...i.flatMap(e=>[e.t,e.beta,e.normalizedT])].every(Number.isFinite)||n<=0||i.length<2||i.some((e,t)=>!Number.isInteger(e.t)||e.t!==t+1||e.beta<=0||e.beta>=1||e.normalizedT<=(i[t-1]?.normalizedT??0)||e.normalizedT>1)||Math.abs(i[i.length-1].normalizedT-1)>1e-12)throw Error(`NN19 V1 needs finite scalars, a positive learning rate, and consecutive increasing diffusion steps ending at normalized time 1.`);let a=ru(r),o=i.map(e=>({...e})),s=iu(e,t,o),c=au(s,t,a),l=c.rows.length,u=c.rows.map(e=>{let t=e.error/l;return{t:e.t,predictionGradient:t,sampleWeightContribution:t*e.noisySample,timestepWeightContribution:t*e.normalizedT,biasContribution:t}}),d=u.reduce((e,t)=>e+t.sampleWeightContribution,0),f=u.reduce((e,t)=>e+t.timestepWeightContribution,0),p=u.reduce((e,t)=>e+t.biasContribution,0),m=[d,f,p],h=[a.sampleWeight,a.timestepWeight,a.bias],g=1e-6,_=h.map((e,n)=>{let r=[...h],i=[...h];r[n]+=g,i[n]-=g;let a=e=>au(s,t,{sampleWeight:e[0],timestepWeight:e[1],bias:e[2]}).meanLoss;return(a(r)-a(i))/(2*g)}),v=Math.max(...m.map((e,t)=>Math.abs(e-_[t]))),y={sampleWeight:a.sampleWeight-n*d,timestepWeight:a.timestepWeight-n*f,bias:a.bias-n*p},b=au(s,t,y),x=ou(s,y),S=x[x.length-1].outputMean;return{cleanSample:e,savedNoise:t,learningRate:n,schedule:o,denoiser:a,forwardSteps:s,initialDenoising:c.rows,initialMeanLoss:c.meanLoss,backward:{perStep:u,sampleWeightGradient:d,timestepWeightGradient:f,biasGradient:p},gradientCheck:{epsilon:g,parameterOrder:[...nu],analytical:m,numerical:_,maxAbsoluteError:v},updatedDenoiser:y,postUpdateDenoising:b.rows,postUpdateMeanLoss:b.meanLoss,reverseSteps:x,finalReconstruction:S,finalAbsoluteError:Math.abs(S-e)}}var cu=[{value:`clean`,shortLabel:`0. Data`,label:`Clean sample`},{value:`forward1`,shortLabel:`1. Forward`,label:`Noise level 1`},{value:`forward2`,shortLabel:`2. Forward`,label:`Noise level 2`},{value:`learn`,shortLabel:`3. Learn`,label:`Predict saved noise`},{value:`reverse2`,shortLabel:`4. Reverse`,label:`Denoise step 2`},{value:`reverse1`,shortLabel:`5. Reverse`,label:`Denoise step 1`}];function G(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function lu(){let[e,t]=(0,l.useState)(`clean`),n=(0,l.useMemo)(()=>su(),[]),r=cu.findIndex(t=>t.value===e),i=r>=3,a=i?n.postUpdateDenoising:n.initialDenoising,o=i?n.postUpdateMeanLoss:n.initialMeanLoss,s=i?n.updatedDenoiser:n.denoiser,c=r>=4,u=r>=5,d=e===`clean`?n.cleanSample:e===`forward1`?n.forwardSteps[0].noisySample:e===`reverse2`?n.reverseSteps[0].outputMean:e===`reverse1`?n.finalReconstruction:n.forwardSteps[1].noisySample;return(0,E.jsxs)(`main`,{className:`workspace workspace--diffusion`,children:[(0,E.jsxs)(`section`,{className:`diffusion-stage`,"aria-label":`One-dimensional diffusion trace`,children:[(0,E.jsxs)(`div`,{className:`diffusion-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN19 - add known noise, then learn to remove it`}),(0,E.jsx)(`h2`,{children:`One clean number through a diffusion round trip`}),(0,E.jsx)(`p`,{children:`Trade signal for one saved noise value at two known levels, train a timestep-aware predictor, and follow its deterministic reverse mean back toward the data.`})]}),(0,E.jsx)(`div`,{className:`diffusion-chip`,children:`x0 -> x1 -> x2 -> mean1 -> mean0`})]}),(0,E.jsxs)(`section`,{className:`diffusion-forward-panel`,"aria-label":`Diffusion forward noise schedule`,children:[(0,E.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One epsilon, two comparable noise levels`}),(0,E.jsx)(`h2`,{children:`Signal shrinks while noise grows`})]}),(0,E.jsxs)(`code`,{children:[`saved epsilon = `,G(n.savedNoise)]})]}),(0,E.jsxs)(`div`,{className:`diffusion-forward-lane`,children:[(0,E.jsxs)(`div`,{className:e===`clean`?`diffusion-state diffusion-state--active`:`diffusion-state`,children:[(0,E.jsx)(`small`,{children:`clean data`}),(0,E.jsxs)(`strong`,{children:[`x0 = `,G(n.cleanSample)]}),(0,E.jsx)(`span`,{children:`100% signal`})]}),n.forwardSteps.map((t,r)=>(0,E.jsxs)(`div`,{className:`diffusion-forward-hop`,children:[(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+ noise`}),(0,E.jsxs)(`div`,{className:e===`forward${t.t}`?`diffusion-state diffusion-state--active diffusion-state--noisy`:`diffusion-state diffusion-state--noisy`,children:[(0,E.jsxs)(`small`,{children:[`noise level `,t.t]}),(0,E.jsxs)(`code`,{children:[G(t.signalScale),` x `,G(n.cleanSample),` + `,G(t.noiseScale),` x (`,G(n.savedNoise),`)`]}),(0,E.jsxs)(`strong`,{children:[`x`,t.t,` = `,G(t.noisySample)]}),(0,E.jsxs)(`span`,{children:[`alpha_bar = `,G(t.alphaBar)]})]})]},t.t))]}),(0,E.jsx)(`div`,{className:`diffusion-coefficient-grid`,children:n.forwardSteps.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`level `,e.t,` contributions`]}),(0,E.jsxs)(`code`,{children:[`signal `,G(e.signalContribution)]}),(0,E.jsxs)(`code`,{children:[`noise `,G(e.noiseContribution)]}),(0,E.jsxs)(`strong`,{children:[G(e.signalContribution),` + `,G(e.noiseContribution),` = `,G(e.noisySample)]})]},e.t))}),(0,E.jsx)(`p`,{className:`diffusion-forward-note`,children:`Each row samples directly from x0 with the same saved epsilon. That makes coefficient changes comparable; it is not one Markov noise path.`})]}),(0,E.jsxs)(`section`,{className:`diffusion-predict-panel`,"aria-label":`Diffusion noise prediction objective`,children:[(0,E.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`The model predicts corruption, not x0 directly`}),(0,E.jsx)(`h2`,{children:`Condition the denoiser on sample and timestep`})]}),(0,E.jsxs)(`div`,{className:`diffusion-loss-badge`,children:[(0,E.jsx)(`small`,{children:i?`mean loss after SGD`:`initial mean loss`}),(0,E.jsx)(`strong`,{children:G(o)})]})]}),(0,E.jsx)(`div`,{className:`diffusion-equation`,children:(0,E.jsxs)(`code`,{children:[`epsilon_hat = `,G(s.sampleWeight),` x x_t + `,G(s.timestepWeight),` x normalized_t + `,G(s.bias)]})}),(0,E.jsx)(`div`,{className:`diffusion-prediction-grid`,children:a.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`level `,e.t,`, normalized t = `,G(e.normalizedT)]}),(0,E.jsxs)(`code`,{children:[`input x`,e.t,` = `,G(e.noisySample)]}),(0,E.jsxs)(`strong`,{children:[`predicted `,G(e.predictedNoise)]}),(0,E.jsxs)(`span`,{children:[`target `,G(e.targetNoise)]}),(0,E.jsxs)(`span`,{children:[`half-squared loss `,G(e.loss)]})]},e.t))})]}),(0,E.jsxs)(`section`,{className:`diffusion-gradient-panel`,"aria-label":`Diffusion denoiser gradient and update`,children:[(0,E.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Both timesteps train one shared denoiser`}),(0,E.jsx)(`h2`,{children:`Add row contributions, audit, then update`})]}),(0,E.jsxs)(`code`,{children:[`parameter - `,G(n.learningRate),` x gradient`]})]}),(0,E.jsx)(`div`,{className:`diffusion-gradient-rows`,children:n.backward.perStep.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`level `,e.t]}),(0,E.jsxs)(`code`,{children:[`dL / d prediction = `,G(e.predictionGradient)]}),(0,E.jsxs)(`span`,{children:[`sample-w route `,G(e.sampleWeightContribution)]}),(0,E.jsxs)(`span`,{children:[`time-w route `,G(e.timestepWeightContribution)]}),(0,E.jsxs)(`span`,{children:[`bias route `,G(e.biasContribution)]})]},e.t))}),(0,E.jsxs)(`div`,{className:`diffusion-gradient-sum`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`sample weight gradient`}),(0,E.jsx)(`strong`,{children:G(n.backward.sampleWeightGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`timestep weight gradient`}),(0,E.jsx)(`strong`,{children:G(n.backward.timestepWeightGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias gradient`}),(0,E.jsx)(`strong`,{children:G(n.backward.biasGradient)})]})]}),(0,E.jsxs)(`div`,{className:`diffusion-update-row`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`parameters before -> after`}),(0,E.jsxs)(`code`,{children:[`sample w `,G(n.denoiser.sampleWeight),` -> `,G(n.updatedDenoiser.sampleWeight)]}),(0,E.jsxs)(`code`,{children:[`time w `,G(n.denoiser.timestepWeight),` -> `,G(n.updatedDenoiser.timestepWeight)]}),(0,E.jsxs)(`code`,{children:[`bias `,G(n.denoiser.bias),` -> `,G(n.updatedDenoiser.bias)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`central finite-difference audit`}),(0,E.jsx)(`strong`,{children:`3 parameters`}),(0,E.jsxs)(`code`,{children:[`max error `,n.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,E.jsxs)(`div`,{className:`diffusion-loss-drop`,children:[(0,E.jsx)(`small`,{children:`same two rows rerun`}),(0,E.jsxs)(`strong`,{children:[G(n.initialMeanLoss),` -> `,G(n.postUpdateMeanLoss)]}),(0,E.jsx)(`span`,{children:`noise prediction improves`})]})]})]}),(0,E.jsxs)(`section`,{className:`diffusion-reverse-panel`,"aria-label":`Diffusion deterministic reverse mean path`,children:[(0,E.jsxs)(`div`,{className:`diffusion-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Subtract predicted noise one level at a time`}),(0,E.jsx)(`h2`,{children:`Run the updated model backward`})]}),(0,E.jsx)(`code`,{children:`no fresh reverse noise in this audit`})]}),(0,E.jsxs)(`div`,{className:`diffusion-reverse-lane`,children:[(0,E.jsxs)(`div`,{className:`diffusion-state diffusion-state--noisy`,children:[(0,E.jsx)(`small`,{children:`start at noisiest sample`}),(0,E.jsxs)(`strong`,{children:[`x2 = `,G(n.forwardSteps[1].noisySample)]})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:e===`reverse2`?`diffusion-reverse-step diffusion-reverse-step--active`:`diffusion-reverse-step`,children:[(0,E.jsx)(`small`,{children:`reverse t = 2`}),c?(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`code`,{children:[G(n.reverseSteps[0].inputSample),` - (`,G(n.reverseSteps[0].scaledNoiseCorrection),`)`]}),(0,E.jsxs)(`strong`,{children:[`mean1 = `,G(n.reverseSteps[0].outputMean)]}),(0,E.jsxs)(`span`,{children:[`predicted noise `,G(n.reverseSteps[0].predictedNoise)]})]}):(0,E.jsx)(`strong`,{children:`?`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:e===`reverse1`?`diffusion-reverse-step diffusion-reverse-step--active`:`diffusion-reverse-step`,children:[(0,E.jsx)(`small`,{children:`reverse t = 1`}),u?(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`code`,{children:[G(n.reverseSteps[1].inputSample),` - (`,G(n.reverseSteps[1].scaledNoiseCorrection),`)`]}),(0,E.jsxs)(`strong`,{children:[`mean0 = `,G(n.finalReconstruction)]}),(0,E.jsxs)(`span`,{children:[`predicted noise `,G(n.reverseSteps[1].predictedNoise)]})]}):(0,E.jsx)(`strong`,{children:`?`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`diffusion-final-state`,children:[(0,E.jsx)(`small`,{children:`reconstructed clean sample`}),(0,E.jsx)(`strong`,{children:u?G(n.finalReconstruction):`?`}),(0,E.jsx)(`span`,{children:u?`absolute error ${G(n.finalAbsoluteError)}`:`finish both reverse means`})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`diffusion-controls`,"aria-label":`Diffusion phase controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Round-trip schedule`}),(0,E.jsx)(`h2`,{children:`Advance the process`}),(0,E.jsx)(`p`,{children:`Forward levels share saved noise. Reverse levels reuse the learned denoiser but feed each generated mean into the next step.`}),(0,E.jsx)(`div`,{className:`diffusion-phase-buttons`,children:cu.map(n=>(0,E.jsxs)(`button`,{type:`button`,"aria-pressed":e===n.value,onClick:()=>t(n.value),children:[(0,E.jsx)(`span`,{children:n.shortLabel}),(0,E.jsx)(`strong`,{children:n.label})]},n.value))}),(0,E.jsxs)(`div`,{className:`diffusion-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected state`}),(0,E.jsx)(`strong`,{children:cu[r].label}),(0,E.jsxs)(`span`,{children:[`visible scalar = `,G(d)]}),(0,E.jsx)(`span`,{children:i?`updated denoiser`:`initial denoiser`})]})]})]})}var uu={generator:{weight:.2,bias:0},discriminator:{weight:1,bias:0}},du=.5,fu=.25;function pu(e){return Math.abs(e)<1e-12?0:e}function mu(e){if(e>=0)return 1/(1+Math.exp(-e));let t=Math.exp(e);return t/(1+t)}function hu(e,t,n){let r=pu(t*n.generator.weight),i=pu(r+n.generator.bias),a=pu(e*n.discriminator.weight+n.discriminator.bias),o=pu(i*n.discriminator.weight+n.discriminator.bias),s=mu(a),c=mu(o);return{generatorProduct:r,fakeSample:i,realLogit:a,realProbability:s,fakeLogit:o,fakeProbability:c,discriminatorLoss:-.5*(Math.log(s)+Math.log(1-c)),generatorLoss:-Math.log(c)}}function gu(e,t){let n=1e-6;return{epsilon:n,numerical:e.map((r,i)=>{let a=[...e],o=[...e];return a[i]+=n,o[i]-=n,(t(a)-t(o))/(2*n)}),maxAbsoluteError:0}}function _u(e,t){return Math.max(...e.map((e,n)=>Math.abs(e-t[n])))}function vu(e=1,t=1,n=du,r=fu,i=uu){if(![e,t,n,r,i.generator.weight,i.generator.bias,i.discriminator.weight,i.discriminator.bias].every(Number.isFinite)||n<=0||r<=0)throw Error(`NN18 V1 needs finite scalar samples and parameters, plus positive learning rates.`);let a={generator:{...i.generator},discriminator:{...i.discriminator}},o=hu(e,t,a),s=.5*(o.realProbability-1),c=.5*o.fakeProbability,l=pu(s*e+c*o.fakeSample),u=pu(s+c),d=[l,u],f=gu([a.discriminator.weight,a.discriminator.bias],([t,n])=>{let r=mu(e*t+n),i=mu(o.fakeSample*t+n);return-.5*(Math.log(r)+Math.log(1-i))}),p={weight:pu(a.discriminator.weight-n*l),bias:pu(a.discriminator.bias-n*u)},m=hu(e,t,{generator:a.generator,discriminator:p}),h=m.fakeProbability-1,g=pu(h*p.weight),_=pu(g*t),v=g,y=[_,v],b=gu([a.generator.weight,a.generator.bias],([e,n])=>{let r=mu((t*e+n)*p.weight+p.bias);return-Math.log(r)}),x={weight:pu(a.generator.weight-r*_),bias:pu(a.generator.bias-r*v)},S=hu(e,t,{generator:x,discriminator:p});return{realSample:e,savedNoise:t,discriminatorLearningRate:n,generatorLearningRate:r,parameters:a,initial:o,discriminatorStep:{backward:{realLogitGradient:s,fakeLogitGradient:c,weightGradient:l,biasGradient:u,fakeSampleGradient:0},updatedParameters:p,state:m,gradientCheck:{epsilon:f.epsilon,parameterOrder:[`discriminator.weight`,`discriminator.bias`],analytical:d,numerical:f.numerical,maxAbsoluteError:_u(d,f.numerical)}},generatorStep:{backward:{fakeLogitGradient:h,fakeSampleGradient:g,weightGradient:_,biasGradient:v},updatedParameters:x,state:S,gradientCheck:{epsilon:b.epsilon,parameterOrder:[`generator.weight`,`generator.bias`],analytical:y,numerical:b.numerical,maxAbsoluteError:_u(y,b.numerical)}}}}var yu=[{value:`initial`,label:`Before training`,shortLabel:`0. Forward`},{value:`discriminator`,label:`Discriminator moves`,shortLabel:`1. Critic`},{value:`generator`,label:`Generator responds`,shortLabel:`2. Maker`}];function K(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function bu(e){return e===`discriminator`?`The fake sample is detached. Only the discriminator can move.`:e===`generator`?`The updated discriminator is frozen. Its input gradient teaches the generator.`:`Both players make predictions, but neither has moved yet.`}function xu({state:e,realSample:t}){let n=e=>`${Math.max(3,Math.min(97,e*72+12))}%`;return(0,E.jsxs)(`div`,{className:`gan-number-line`,"aria-label":`GAN sample number line`,children:[(0,E.jsxs)(`div`,{className:`gan-number-line__axis`,"aria-hidden":`true`,children:[(0,E.jsx)(`span`,{children:`0`}),(0,E.jsx)(`span`,{children:`0.5`}),(0,E.jsx)(`span`,{children:`1`})]}),(0,E.jsxs)(`div`,{className:`gan-number-line__marker gan-number-line__marker--fake`,style:{left:n(e.fakeSample)},children:[(0,E.jsxs)(`strong`,{children:[`fake `,K(e.fakeSample)]}),(0,E.jsx)(`small`,{children:`G(noise)`})]}),(0,E.jsxs)(`div`,{className:`gan-number-line__marker gan-number-line__marker--real`,style:{left:n(t)},children:[(0,E.jsxs)(`strong`,{children:[`real `,K(t)]}),(0,E.jsx)(`small`,{children:`data`})]})]})}function Su(){let[e,t]=(0,l.useState)(`initial`),n=(0,l.useMemo)(()=>vu(),[]),r=e===`initial`?n.initial:e===`discriminator`?n.discriminatorStep.state:n.generatorStep.state,i=e===`initial`?n.parameters.discriminator:n.discriminatorStep.updatedParameters,a=e===`generator`?n.generatorStep.updatedParameters:n.parameters.generator;return(0,E.jsxs)(`main`,{className:`workspace workspace--gan`,children:[(0,E.jsxs)(`section`,{className:`gan-stage`,"aria-label":`One-dimensional GAN game trace`,children:[(0,E.jsxs)(`div`,{className:`gan-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN18 - two losses, two turns, one game`}),(0,E.jsx)(`h2`,{children:`A generator and discriminator on one number line`}),(0,E.jsx)(`p`,{children:`The critic learns to separate one real point from one generated point. Then the maker follows the frozen critic's slope toward a more convincing sample.`})]}),(0,E.jsx)(`div`,{className:`gan-chip`,children:`D moves -> freeze D -> G moves`})]}),(0,E.jsxs)(`section`,{className:`gan-sample-panel`,"aria-label":`GAN samples and discriminator probabilities`,children:[(0,E.jsxs)(`div`,{className:`gan-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same saved noise through every phase`}),(0,E.jsx)(`h2`,{children:`Watch the fake sample move toward the data`})]}),(0,E.jsx)(`code`,{children:bu(e)})]}),(0,E.jsx)(xu,{state:r,realSample:n.realSample}),(0,E.jsxs)(`div`,{className:`gan-probability-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`critic on real`}),(0,E.jsxs)(`code`,{children:[`sigmoid(`,K(r.realLogit),`)`]}),(0,E.jsx)(`strong`,{children:K(r.realProbability)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`critic on fake`}),(0,E.jsxs)(`code`,{children:[`sigmoid(`,K(r.fakeLogit),`)`]}),(0,E.jsx)(`strong`,{children:K(r.fakeProbability)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`generator equation`}),(0,E.jsxs)(`code`,{children:[K(n.savedNoise),` x `,K(a.weight),` + `,K(a.bias)]}),(0,E.jsx)(`strong`,{children:K(r.fakeSample)})]})]})]}),(0,E.jsxs)(`section`,{className:`gan-objective-panel`,"aria-label":`GAN competing objectives`,children:[(0,E.jsxs)(`div`,{className:`gan-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`The players do not minimize one shared loss`}),(0,E.jsx)(`h2`,{children:`Judge correctly; fool the judge`})]}),(0,E.jsx)(`code`,{children:`non-saturating generator objective`})]}),(0,E.jsxs)(`div`,{className:`gan-objectives`,children:[(0,E.jsxs)(`div`,{className:e===`discriminator`?`gan-player gan-player--active`:`gan-player`,children:[(0,E.jsx)(`small`,{children:`discriminator minimizes`}),(0,E.jsx)(`code`,{children:`-0.5 x [log D(real) + log(1 - D(fake))]`}),(0,E.jsxs)(`strong`,{children:[`D loss `,K(r.discriminatorLoss)]}),(0,E.jsx)(`span`,{children:`real label 1, fake label 0`})]}),(0,E.jsx)(`div`,{className:`gan-versus`,"aria-hidden":`true`,children:`vs`}),(0,E.jsxs)(`div`,{className:e===`generator`?`gan-player gan-player--active gan-player--generator`:`gan-player gan-player--generator`,children:[(0,E.jsx)(`small`,{children:`generator minimizes`}),(0,E.jsx)(`code`,{children:`-log D(G(noise))`}),(0,E.jsxs)(`strong`,{children:[`G loss `,K(r.generatorLoss)]}),(0,E.jsx)(`span`,{children:`make the fake receive label 1`})]})]})]}),(0,E.jsxs)(`section`,{className:`gan-gradient-panel`,"aria-label":`GAN active gradient route`,children:[(0,E.jsxs)(`div`,{className:`gan-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Only one parameter set moves per turn`}),(0,E.jsx)(`h2`,{children:e===`generator`?`The critic becomes a teaching signal`:e===`discriminator`?`The generated value is detached`:`Choose a move to reveal its gradient`})]}),(0,E.jsx)(`code`,{children:e===`initial`?`forward pass only`:`active route highlighted`})]}),e===`initial`?(0,E.jsx)(`div`,{className:`gan-gradient-placeholder`,children:`Start with two sigmoid scores. The turn buttons expose which edges carry gradients and which parameter set stays frozen.`}):e===`discriminator`?(0,E.jsxs)(`div`,{className:`gan-gradient-route`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`real-logit route`}),(0,E.jsx)(`code`,{children:`0.5 x (D(real) - 1)`}),(0,E.jsx)(`strong`,{children:K(n.discriminatorStep.backward.realLogitGradient)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`fake-logit route`}),(0,E.jsx)(`code`,{children:`0.5 x D(fake)`}),(0,E.jsx)(`strong`,{children:K(n.discriminatorStep.backward.fakeLogitGradient)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`gan-gradient-route__result`,children:[(0,E.jsx)(`small`,{children:`D weight / bias gradient`}),(0,E.jsxs)(`strong`,{children:[K(n.discriminatorStep.backward.weightGradient),` / `,K(n.discriminatorStep.backward.biasGradient)]}),(0,E.jsx)(`span`,{children:`gradient into fake = 0 (detached)`})]})]}):(0,E.jsxs)(`div`,{className:`gan-gradient-route`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`G loss to fake logit`}),(0,E.jsx)(`code`,{children:`D(fake) - 1`}),(0,E.jsx)(`strong`,{children:K(n.generatorStep.backward.fakeLogitGradient)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`x`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`frozen D input slope`}),(0,E.jsxs)(`code`,{children:[`D weight = `,K(i.weight)]}),(0,E.jsx)(`strong`,{children:K(n.generatorStep.backward.fakeSampleGradient)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`gan-gradient-route__result gan-gradient-route__result--generator`,children:[(0,E.jsx)(`small`,{children:`G weight / bias gradient`}),(0,E.jsxs)(`strong`,{children:[K(n.generatorStep.backward.weightGradient),` / `,K(n.generatorStep.backward.biasGradient)]}),(0,E.jsx)(`span`,{children:`D parameters stay frozen`})]})]})]}),(0,E.jsxs)(`section`,{className:`gan-update-panel`,"aria-label":`GAN alternating updates and gradient audits`,children:[(0,E.jsxs)(`div`,{className:`gan-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Audit each player against its own objective`}),(0,E.jsx)(`h2`,{children:`The losses push back after alternating moves`})]}),(0,E.jsx)(`code`,{children:`central difference epsilon = 1e-6`})]}),(0,E.jsxs)(`div`,{className:`gan-update-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`discriminator update`}),(0,E.jsxs)(`code`,{children:[`w `,K(n.parameters.discriminator.weight),` -> `,K(n.discriminatorStep.updatedParameters.weight)]}),(0,E.jsxs)(`code`,{children:[`b `,K(n.parameters.discriminator.bias),` -> `,K(n.discriminatorStep.updatedParameters.bias)]}),(0,E.jsxs)(`strong`,{children:[`D loss `,K(n.initial.discriminatorLoss),` -> `,K(n.discriminatorStep.state.discriminatorLoss)]}),(0,E.jsxs)(`span`,{children:[`max audit error `,n.discriminatorStep.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`generator counter-move`}),(0,E.jsxs)(`code`,{children:[`w `,K(n.parameters.generator.weight),` -> `,K(n.generatorStep.updatedParameters.weight)]}),(0,E.jsxs)(`code`,{children:[`b `,K(n.parameters.generator.bias),` -> `,K(n.generatorStep.updatedParameters.bias)]}),(0,E.jsxs)(`strong`,{children:[`G loss `,K(n.discriminatorStep.state.generatorLoss),` -> `,K(n.generatorStep.state.generatorLoss)]}),(0,E.jsxs)(`span`,{children:[`max audit error `,n.generatorStep.gradientCheck.maxAbsoluteError.toExponential(3)]})]})]}),(0,E.jsxs)(`div`,{className:`gan-counterpush`,children:[(0,E.jsxs)(`strong`,{children:[`After G moves, D loss rises to `,K(n.generatorStep.state.discriminatorLoss),`.`]}),(0,E.jsx)(`p`,{children:`That is the game working: the newly improved fake is harder for the frozen critic.`})]})]})]}),(0,E.jsxs)(`aside`,{className:`gan-controls`,"aria-label":`GAN game phase controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Alternating schedule`}),(0,E.jsx)(`h2`,{children:`Advance one turn`}),(0,E.jsx)(`p`,{children:`These are snapshots of one deterministic round, not three independent experiments.`}),(0,E.jsx)(`div`,{className:`gan-phase-buttons`,children:yu.map(n=>(0,E.jsxs)(`button`,{type:`button`,"aria-pressed":e===n.value,onClick:()=>t(n.value),children:[(0,E.jsx)(`span`,{children:n.shortLabel}),(0,E.jsx)(`strong`,{children:n.label})]},n.value))}),(0,E.jsxs)(`div`,{className:`gan-selected-summary`,children:[(0,E.jsx)(`small`,{children:`current snapshot`}),(0,E.jsx)(`strong`,{children:yu.find(t=>t.value===e).label}),(0,E.jsxs)(`span`,{children:[`fake = `,K(r.fakeSample)]}),(0,E.jsxs)(`span`,{children:[`D(fake) = `,K(r.fakeProbability)]})]}),(0,E.jsxs)(`div`,{className:`gan-freeze-key`,children:[(0,E.jsx)(`small`,{children:`freeze contract`}),(0,E.jsx)(`code`,{children:e===`discriminator`?`grad(G) = 0`:e===`generator`?`grad(D params) = 0`:`no backward pass`})]})]})]})}var Cu={encoder:{mean:{weight:.4,bias:0},logVariance:{weight:0,bias:0}},decoder:{weight:1,bias:0}},wu=.5,Tu=.1,Eu=.1,Du=[`encoder.mean.weight`,`encoder.mean.bias`,`encoder.log_variance.weight`,`encoder.log_variance.bias`,`decoder.weight`,`decoder.bias`];function Ou(e){return Math.abs(e)<1e-12?0:e}function ku(e){return{encoder:{mean:{...e.encoder.mean},logVariance:{...e.encoder.logVariance}},decoder:{...e.decoder}}}function Au(e,t,n,r){let i=Ou(e*t.encoder.mean.weight),a=Ou(i+t.encoder.mean.bias),o=Ou(e*t.encoder.logVariance.weight),s=Ou(o+t.encoder.logVariance.bias),c=Math.exp(s),l=Math.exp(.5*s),u=Ou(l*n),d=Ou(a+u),f=Ou(d*t.decoder.weight),p=Ou(f+t.decoder.bias),m=Ou(p-e),h=.5*m*m,g=a*a,_=.5*(g+c-1-s),v=r*_;return{meanProduct:i,mean:a,logVarianceProduct:o,logVariance:s,variance:c,standardDeviation:l,epsilon:n,noiseContribution:u,latent:d,decoderProduct:f,reconstruction:p,error:m,reconstructionLoss:h,meanSquared:g,kl:_,weightedKl:v,totalLoss:h+v}}function ju(e){return[e.encoder.mean.weight,e.encoder.mean.bias,e.encoder.logVariance.weight,e.encoder.logVariance.bias,e.decoder.weight,e.decoder.bias]}function Mu(e){return{encoder:{mean:{weight:e[0],bias:e[1]},logVariance:{weight:e[2],bias:e[3]}},decoder:{weight:e[4],bias:e[5]}}}function Nu(e=Tu,t=wu,n=Eu,r=1,i=Cu){let a=ju(i);if(!Number.isFinite(e)||e<0||!Number.isFinite(t)||!Number.isFinite(n)||n<=0||!Number.isFinite(r)||!a.every(Number.isFinite))throw Error(`NN17 V1 needs finite scalar parameters, input and epsilon, non-negative beta, and a positive learning rate.`);let o=ku(i),s=Au(r,o,t,e);if(!Number.isFinite(s.variance)||!Number.isFinite(s.standardDeviation)||!Number.isFinite(s.totalLoss))throw Error(`NN17 V1 produced a non-finite Gaussian or objective.`);let c=s.error,l=Ou(c*s.latent),u=c,d=Ou(c*o.decoder.weight),f=d,p=Ou(d*.5*s.standardDeviation*t),m=s.mean,h=Ou(.5*(s.variance-1)),g=Ou(e*m),_=Ou(e*h),v=Ou(f+g),y=Ou(p+_),b=Ou(v*r),x=v,S=Ou(y*r),C=y,w={reconstructionGradient:c,decoderWeightGradient:l,decoderBiasGradient:u,latentGradient:d,reconstructionMeanGradient:f,reconstructionLogVarianceGradient:p,klMeanGradient:m,klLogVarianceGradient:h,weightedKlMeanGradient:g,weightedKlLogVarianceGradient:_,meanGradient:v,logVarianceGradient:y,meanWeightGradient:b,meanBiasGradient:x,logVarianceWeightGradient:S,logVarianceBiasGradient:C},ee=[b,x,S,C,l,u],te=1e-6,T=a.map((n,i)=>{let o=[...a],s=[...a];return o[i]+=te,s[i]-=te,(Au(r,Mu(o),t,e).totalLoss-Au(r,Mu(s),t,e).totalLoss)/(2*te)}),ne=Math.max(...ee.map((e,t)=>Math.abs(e-T[t]))),re=Mu(a.map((e,t)=>e-n*ee[t])),ie=Au(r,re,t,e);return{input:r,beta:e,samplingEpsilon:t,learningRate:n,parameters:o,forward:s,backward:w,gradientCheck:{epsilon:te,parameterOrder:[...Du],analytical:ee,numerical:T,maxAbsoluteError:ne},updatedParameters:re,postUpdate:ie}}var Pu=[0,.1,.25,1];function q(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(8)).toString()}function Fu(){let[e,t]=(0,l.useState)(.1),[n,r]=(0,l.useState)(`mean`),[i,a]=(0,l.useState)(!1),o=(0,l.useMemo)(()=>Nu(e),[e]),s=i?o.postUpdate:o.forward,c=i?o.updatedParameters:o.parameters,u=n===`mean`?o.backward.reconstructionMeanGradient:o.backward.reconstructionLogVarianceGradient,d=n===`mean`?o.backward.weightedKlMeanGradient:o.backward.weightedKlLogVarianceGradient,f=n===`mean`?o.backward.meanGradient:o.backward.logVarianceGradient,p=n===`mean`?`mean`:`log-variance`;return(0,E.jsxs)(`main`,{className:`workspace workspace--variational`,children:[(0,E.jsxs)(`section`,{className:`variational-stage`,"aria-label":`Scalar variational autoencoder trace`,children:[(0,E.jsxs)(`div`,{className:`variational-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN17 - uncertainty without hidden randomness`}),(0,E.jsx)(`h2`,{children:`One Gaussian latent sample, fully unpacked`}),(0,E.jsx)(`p`,{children:`Encode a mean and log-variance, transform one saved noise value, then watch reconstruction and prior matching negotiate one update.`})]}),(0,E.jsx)(`div`,{className:`variational-chip`,children:`mean + sigma x epsilon`})]}),(0,E.jsxs)(`section`,{className:`variational-flow-panel`,"aria-label":`Variational encode sample and decode path`,children:[(0,E.jsxs)(`div`,{className:`variational-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`The sample is random; the path is differentiable`}),(0,E.jsx)(`h2`,{children:`Move noise outside the network`})]}),(0,E.jsx)(`code`,{children:i?`after one SGD step`:`saved epsilon = 0.5`})]}),(0,E.jsxs)(`div`,{className:`variational-flow`,children:[(0,E.jsxs)(`div`,{className:`variational-scalar-node`,children:[(0,E.jsx)(`small`,{children:`input is target`}),(0,E.jsxs)(`strong`,{children:[`x = `,q(o.input)]})]}),(0,E.jsx)(`span`,{className:`variational-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`variational-distribution-node`,children:[(0,E.jsx)(`small`,{children:`encoder distribution`}),(0,E.jsxs)(`code`,{children:[`mean = `,q(s.meanProduct),` + `,q(c.encoder.mean.bias),` = `,q(s.mean)]}),(0,E.jsxs)(`code`,{children:[`log var = `,q(s.logVarianceProduct),` + `,q(c.encoder.logVariance.bias),` = `,q(s.logVariance)]}),(0,E.jsxs)(`code`,{children:[`sigma = `,q(s.standardDeviation)]})]}),(0,E.jsx)(`span`,{className:`variational-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`variational-sample-node`,children:[(0,E.jsx)(`small`,{children:`reparameterized sample`}),(0,E.jsxs)(`code`,{children:[q(s.mean),` + `,q(s.standardDeviation),` x `,q(s.epsilon)]}),(0,E.jsxs)(`strong`,{children:[`z = `,q(s.latent)]}),(0,E.jsx)(`span`,{children:`epsilon stays fixed for this audit`})]}),(0,E.jsx)(`span`,{className:`variational-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{className:`variational-scalar-node variational-scalar-node--output`,children:[(0,E.jsx)(`small`,{children:`decoder reconstruction`}),(0,E.jsxs)(`code`,{children:[q(s.latent),` x `,q(c.decoder.weight),` + `,q(c.decoder.bias)]}),(0,E.jsxs)(`strong`,{children:[`x_hat = `,q(s.reconstruction)]})]})]})]}),(0,E.jsxs)(`section`,{className:`variational-objective-panel`,"aria-label":`Variational reconstruction and KL objective`,children:[(0,E.jsxs)(`div`,{className:`variational-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Two pressures, one weighted objective`}),(0,E.jsx)(`h2`,{children:`Reconstruct here; stay sampleable everywhere`})]}),(0,E.jsxs)(`div`,{className:`variational-loss-badge`,children:[(0,E.jsx)(`small`,{children:`total loss`}),(0,E.jsx)(`strong`,{children:q(s.totalLoss)})]})]}),(0,E.jsxs)(`div`,{className:`variational-objective-equation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`reconstruction`}),(0,E.jsxs)(`code`,{children:[`0.5 x (`,q(s.error),`)^2`]}),(0,E.jsx)(`strong`,{children:q(s.reconstructionLoss)}),(0,E.jsx)(`span`,{children:`preserve this input`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`KL to Normal(0, 1)`}),(0,E.jsxs)(`code`,{children:[`0.5 x (`,q(s.meanSquared),` + `,q(s.variance),` - 1 - `,q(s.logVariance),`)`]}),(0,E.jsx)(`strong`,{children:q(s.kl)}),(0,E.jsx)(`span`,{children:`keep latent space sampleable`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`x`}),(0,E.jsxs)(`div`,{className:`variational-beta-node`,children:[(0,E.jsx)(`small`,{children:`beta`}),(0,E.jsx)(`strong`,{children:q(e)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,E.jsxs)(`div`,{className:`variational-total-node`,children:[(0,E.jsx)(`small`,{children:`weighted total`}),(0,E.jsxs)(`code`,{children:[q(s.reconstructionLoss),` + `,q(s.weightedKl)]}),(0,E.jsx)(`strong`,{children:q(s.totalLoss)})]})]})]}),(0,E.jsxs)(`section`,{className:`variational-gradient-panel`,"aria-label":`Variational ${p} gradient tradeoff`,children:[(0,E.jsxs)(`div`,{className:`variational-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Both objectives meet at the encoder`}),(0,E.jsx)(`h2`,{children:`Beta can reinforce, soften, or reverse a direction`})]}),(0,E.jsx)(`code`,{children:`saved forward pass gradients`})]}),(0,E.jsxs)(`div`,{className:`variational-gradient-targets`,"aria-label":`Variational gradient target`,children:[(0,E.jsx)(`button`,{"aria-pressed":n===`mean`,type:`button`,onClick:()=>r(`mean`),children:`mean output`}),(0,E.jsx)(`button`,{"aria-pressed":n===`logVariance`,type:`button`,onClick:()=>r(`logVariance`),children:`log-variance output`})]}),(0,E.jsxs)(`div`,{className:`variational-gradient-routes`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`reconstruction route`}),(0,E.jsx)(`strong`,{children:q(u)}),(0,E.jsx)(`span`,{children:`sample should rebuild x`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`beta x KL route`}),(0,E.jsxs)(`code`,{children:[q(e),` x `,q(n===`mean`?o.backward.klMeanGradient:o.backward.klLogVarianceGradient)]}),(0,E.jsx)(`strong`,{children:q(d)}),(0,E.jsx)(`span`,{children:`distribution should match prior`})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`=`}),(0,E.jsxs)(`div`,{className:`variational-combined-gradient`,children:[(0,E.jsxs)(`small`,{children:[`combined `,p,` gradient`]}),(0,E.jsx)(`strong`,{children:q(f)}),(0,E.jsx)(`span`,{children:f===0?`the routes cancel exactly`:`this is the encoder's update direction`})]})]}),(0,E.jsxs)(`div`,{className:`variational-gradient-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder weight`}),(0,E.jsx)(`code`,{children:q(o.backward.decoderWeightGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder bias`}),(0,E.jsx)(`code`,{children:q(o.backward.decoderBiasGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean weight / bias`}),(0,E.jsxs)(`code`,{children:[q(o.backward.meanWeightGradient),` / `,q(o.backward.meanBiasGradient)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`log-var weight / bias`}),(0,E.jsxs)(`code`,{children:[q(o.backward.logVarianceWeightGradient),` / `,q(o.backward.logVarianceBiasGradient)]})]})]})]}),(0,E.jsxs)(`section`,{className:`variational-update-panel`,"aria-label":`Variational SGD update and gradient audit`,children:[(0,E.jsxs)(`div`,{className:`variational-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Same epsilon for analytical and numerical slopes`}),(0,E.jsx)(`h2`,{children:`Audit six parameters, then rerun everything`})]}),(0,E.jsxs)(`code`,{children:[`parameter - `,o.learningRate,` x gradient`]})]}),(0,E.jsxs)(`div`,{className:`variational-parameter-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`mean head before -> after`}),(0,E.jsxs)(`code`,{children:[`w `,q(o.parameters.encoder.mean.weight),` -> `,q(o.updatedParameters.encoder.mean.weight)]}),(0,E.jsxs)(`code`,{children:[`b `,q(o.parameters.encoder.mean.bias),` -> `,q(o.updatedParameters.encoder.mean.bias)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`log-var head before -> after`}),(0,E.jsxs)(`code`,{children:[`w `,q(o.parameters.encoder.logVariance.weight),` -> `,q(o.updatedParameters.encoder.logVariance.weight)]}),(0,E.jsxs)(`code`,{children:[`b `,q(o.parameters.encoder.logVariance.bias),` -> `,q(o.updatedParameters.encoder.logVariance.bias)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`decoder before -> after`}),(0,E.jsxs)(`code`,{children:[`w `,q(o.parameters.decoder.weight),` -> `,q(o.updatedParameters.decoder.weight)]}),(0,E.jsxs)(`code`,{children:[`b `,q(o.parameters.decoder.bias),` -> `,q(o.updatedParameters.decoder.bias)]})]})]}),(0,E.jsxs)(`div`,{className:`variational-audit-row`,children:[(0,E.jsx)(`span`,{children:`Central finite differences - 6 parameters`}),(0,E.jsxs)(`code`,{children:[`epsilon = `,o.gradientCheck.epsilon]}),(0,E.jsxs)(`strong`,{children:[`max error `,o.gradientCheck.maxAbsoluteError.toExponential(3)]})]}),(0,E.jsxs)(`div`,{className:`variational-loss-drop`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`total before`}),(0,E.jsx)(`strong`,{children:q(o.forward.totalLoss)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`total after`}),(0,E.jsx)(`strong`,{children:q(o.postUpdate.totalLoss)})]}),(0,E.jsxs)(`p`,{children:[`Reconstruction falls from `,q(o.forward.reconstructionLoss),` to `,q(o.postUpdate.reconstructionLoss),`; KL may move differently while the selected weighted objective falls.`]})]})]})]}),(0,E.jsxs)(`aside`,{className:`variational-controls`,"aria-label":`Variational trace controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Turn the prior pressure`}),(0,E.jsx)(`h2`,{children:`KL tradeoff controls`}),(0,E.jsx)(`p`,{children:`Epsilon stays fixed at 0.5. Changing beta therefore changes the objective and gradient, not the sampled noise.`}),(0,E.jsx)(`div`,{className:`variational-beta-buttons`,"aria-label":`Variational beta selection`,children:Pu.map(n=>(0,E.jsxs)(`button`,{"aria-pressed":e===n,type:`button`,onClick:()=>{t(n),a(!1)},children:[`beta `,n]},n))}),(0,E.jsxs)(`label`,{className:`attention-scale-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:i,onChange:e=>a(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Use updated parameters`}),(0,E.jsx)(`small`,{children:`Rerun distribution, sample, decoder, and both losses.`})]})]}),(0,E.jsxs)(`div`,{className:`variational-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected beta`}),(0,E.jsx)(`strong`,{children:q(e)}),(0,E.jsxs)(`span`,{children:[`mean gradient `,q(o.backward.meanGradient),`; total `,q(o.forward.totalLoss)]})]}),(0,E.jsxs)(`div`,{className:`attention-value-boundary`,children:[(0,E.jsx)(`span`,{children:`Why save epsilon?`}),(0,E.jsx)(`p`,{children:`The trace remains stochastic in meaning but reproducible in execution. Finite differences compare the same noise on both sides.`})]}),(0,E.jsxs)(`div`,{className:`attention-next-note`,children:[(0,E.jsx)(`span`,{children:`Do not optimize one term alone`}),(0,E.jsx)(`p`,{children:`A useful VAE needs reconstruction and a navigable latent prior. Their weighted sum, not either isolated term, defines this step.`})]})]})]})}function Iu(){let[e,t]=(0,l.useState)(`autoencoder`);return(0,E.jsxs)(`div`,{className:`representation-workbench`,children:[(0,E.jsxs)(`nav`,{className:`representation-lab-switch`,"aria-label":`Representation learning lab`,children:[(0,E.jsx)(`button`,{"aria-pressed":e===`autoencoder`,type:`button`,onClick:()=>t(`autoencoder`),children:`Deterministic bottleneck`}),(0,E.jsx)(`button`,{"aria-pressed":e===`variational`,type:`button`,onClick:()=>t(`variational`),children:`Variational sample`}),(0,E.jsx)(`button`,{"aria-pressed":e===`gan`,type:`button`,onClick:()=>t(`gan`),children:`Adversarial game`}),(0,E.jsx)(`button`,{"aria-pressed":e===`diffusion`,type:`button`,onClick:()=>t(`diffusion`),children:`Diffusion path`})]}),e===`autoencoder`?(0,E.jsx)(Zl,{}):e===`variational`?(0,E.jsx)(Fu,{}):e===`gan`?(0,E.jsx)(Su,{}):(0,E.jsx)(lu,{})]})}var Lu=[1,0,2,0,1],Ru=[[1,1,1],[1,1,1]];function zu(e){return e===0?0:e}function Bu(e,t){if(e.length===0||t.length===0||t.length%2==0||![...e,...t].every(Number.isFinite))throw Error(`Same correlation needs a finite signal and an odd kernel.`);let n=Math.floor(t.length/2);return e.map((r,i)=>zu(t.reduce((t,r,a)=>{let o=i+a-n;return t+(o>=0&&o<e.length?e[o]:0)*r},0)))}function Vu(e=Lu,t=Ru){if(t.length!==2||t.some(e=>e.length!==3||e.some(e=>e!==1)))throw Error(`NN08 V1 uses two [1, 1, 1] kernels.`);let n=Bu(e,t[0]),r=Bu(n,t[1]),i=[...e],a=r.map((e,t)=>zu(e+i[t])),o=a.map(e=>Math.max(0,e));return{hidden:n,main:r,skip:i,residualSum:a,output:o,traces:e.map((t,s)=>{let c=[s-1,s,s+1].filter(t=>t>=0&&t<e.length),l=e.map(()=>0),u=c.map(t=>{let r=[t-1,t,t+1].filter(t=>t>=0&&t<e.length);return r.forEach(e=>{l[e]=l[e]+1}),{hiddenIndex:t,inputIndices:r,inputValues:r.map(t=>e[t]),subtotal:n[t]}});return{outputIndex:s,hiddenIndices:c,hiddenValues:c.map(e=>n[e]),hiddenPaths:u,inputPathCounts:l,inputContributions:e.map((e,t)=>zu(e*l[t])),receptiveFieldIndices:l.map((e,t)=>({count:e,inputIndex:t})).filter(({count:e})=>e>0).map(({inputIndex:e})=>e),mainOutput:r[s],skipContribution:i[s],residualSum:a[s],output:o[s]}})}}function Hu(e){return Math.abs(e)<1e-12?`0`:Number(e.toFixed(4)).toString()}function Uu({label:e,values:t,selectedIndex:n,activeIndices:r=[],annotation:i}){return(0,E.jsxs)(`div`,{className:`residual-signal-block`,children:[(0,E.jsxs)(`div`,{className:`residual-row-label`,children:[(0,E.jsx)(`span`,{children:e}),i===void 0?null:(0,E.jsx)(`code`,{children:i})]}),(0,E.jsx)(`div`,{className:`residual-signal-row`,style:{gridTemplateColumns:`repeat(${t.length}, minmax(52px, 1fr))`},"aria-label":e,children:t.map((e,t)=>(0,E.jsxs)(`div`,{className:t===n?`residual-cell residual-cell--selected`:r.includes(t)?`residual-cell residual-cell--active`:`residual-cell`,children:[(0,E.jsxs)(`small`,{children:[`[`,t,`]`]}),(0,E.jsx)(`strong`,{children:Hu(e)})]},t))})]})}function Wu(){let[e,t]=(0,l.useState)(2),[n,r]=(0,l.useState)(!0),i=(0,l.useMemo)(()=>Vu(),[]),a=i.traces[e],o=a.mainOutput+(n?a.skipContribution:0),s=Math.max(0,o),c=i.main.map((e,t)=>Math.max(0,e+(n?i.skip[t]:0)));function u(){t(2),r(!0)}return(0,E.jsxs)(`main`,{className:`workspace workspace--residual`,children:[(0,E.jsxs)(`section`,{className:`residual-stage`,"aria-label":`Residual path and receptive field trace`,children:[(0,E.jsxs)(`div`,{className:`residual-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN08 · spatial networks`}),(0,E.jsx)(`h2`,{children:`Residual-path microscope`}),(0,E.jsx)(`p`,{children:`Open one output into its deep local path and short identity path, then trace every dependency back to the original input.`})]}),(0,E.jsx)(`div`,{className:`residual-shape-chip`,children:`5 → 5 → 5 + identity`})]}),(0,E.jsxs)(`section`,{className:`residual-block-panel`,"aria-label":`Residual block forward trace`,children:[(0,E.jsxs)(`div`,{className:`residual-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Selected output · y[`,e,`]`]}),(0,E.jsx)(`h2`,{children:`Two routes meet at one addition`})]}),(0,E.jsx)(`strong`,{className:`residual-result`,children:Hu(s)})]}),(0,E.jsxs)(`div`,{className:`residual-main-path`,children:[(0,E.jsx)(`span`,{className:`residual-lane-label`,children:`main path · two local layers`}),(0,E.jsx)(Uu,{label:`input x`,values:Lu,selectedIndex:n?e:void 0,activeIndices:a.receptiveFieldIndices,annotation:`receptive field highlighted`}),(0,E.jsx)(`span`,{className:`residual-down-arrow`,"aria-hidden":`true`,children:`↓ [1, 1, 1] · same zero pad`}),(0,E.jsx)(Uu,{label:`hidden h`,values:i.hidden,activeIndices:a.hiddenIndices,annotation:`${a.hiddenIndices.length} values feed main[${e}]`}),(0,E.jsx)(`span`,{className:`residual-down-arrow`,"aria-hidden":`true`,children:`↓ [1, 1, 1] · same zero pad`}),(0,E.jsx)(Uu,{label:`main transform F(x)`,values:i.main,selectedIndex:e,annotation:`main[${e}] = ${Hu(a.mainOutput)}`})]}),(0,E.jsxs)(`div`,{className:n?`residual-skip-lane`:`residual-skip-lane residual-skip-lane--disabled`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`identity skip`}),(0,E.jsxs)(`strong`,{children:[`x[`,e,`] = `,Hu(a.skipContribution)]})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`────────────→`}),(0,E.jsx)(`code`,{children:n?`included`:`disabled`})]}),(0,E.jsxs)(`div`,{className:`residual-addition`,"aria-label":`Selected residual addition`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`main path`}),(0,E.jsx)(`strong`,{children:Hu(a.mainOutput)})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`skip path`}),(0,E.jsx)(`strong`,{children:n?Hu(a.skipContribution):`0`})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`before ReLU`}),(0,E.jsx)(`strong`,{children:Hu(o)})]}),(0,E.jsx)(`span`,{children:`→`}),(0,E.jsxs)(`div`,{className:`residual-addition__output`,children:[(0,E.jsx)(`small`,{children:`output`}),(0,E.jsx)(`strong`,{children:Hu(s)})]})]}),(0,E.jsx)(Uu,{label:n?`block output ReLU(F(x) + x)`:`block output ReLU(F(x))`,values:c,selectedIndex:e})]}),(0,E.jsxs)(`section`,{className:`receptive-panel`,"aria-label":`Receptive field explorer`,children:[(0,E.jsxs)(`div`,{className:`residual-panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Receptive field · output `,e]}),(0,E.jsx)(`h2`,{children:`One output, every path back`})]}),(0,E.jsxs)(`div`,{className:`field-width-badge`,children:[(0,E.jsx)(`small`,{children:`in-range width`}),(0,E.jsx)(`strong`,{children:a.receptiveFieldIndices.length})]})]}),(0,E.jsx)(`div`,{className:`hidden-path-grid`,children:a.hiddenPaths.map(e=>(0,E.jsxs)(`article`,{className:`hidden-path-card`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`layer 2 reads`}),(0,E.jsxs)(`strong`,{children:[`h[`,e.hiddenIndex,`] = `,Hu(e.subtotal)]})]}),(0,E.jsx)(`code`,{children:e.inputIndices.map(e=>`x[${e}]`).join(` + `)}),(0,E.jsxs)(`span`,{children:[e.inputValues.map(Hu).join(` + `),` = `,Hu(e.subtotal)]})]},e.hiddenIndex))}),(0,E.jsx)(`div`,{className:`path-count-table-wrap`,children:(0,E.jsxs)(`table`,{className:`path-count-table`,children:[(0,E.jsx)(`caption`,{children:`Original inputs after expanding both layers`}),(0,E.jsx)(`thead`,{children:(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`col`,children:`input`}),Lu.map((e,t)=>(0,E.jsxs)(`th`,{scope:`col`,children:[`x[`,t,`]`]},t)),(0,E.jsx)(`th`,{scope:`col`,children:`sum`})]})}),(0,E.jsxs)(`tbody`,{children:[(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`paths`}),a.inputPathCounts.map((e,t)=>(0,E.jsx)(`td`,{children:e},t)),(0,E.jsx)(`td`,{children:`—`})]}),(0,E.jsxs)(`tr`,{children:[(0,E.jsx)(`th`,{scope:`row`,children:`value × paths`}),a.inputContributions.map((e,t)=>(0,E.jsx)(`td`,{children:Hu(e)},t)),(0,E.jsx)(`td`,{className:`path-count-total`,children:Hu(a.mainOutput)})]})]})]})}),(0,E.jsxs)(`div`,{className:`receptive-summary`,children:[(0,E.jsxs)(`code`,{children:[`receptive input indices = [`,a.receptiveFieldIndices.join(`, `),`]`]}),(0,E.jsx)(`span`,{children:`Zero-valued inputs still belong to the structural field: changing them can change this output.`})]})]})]}),(0,E.jsxs)(`aside`,{className:`residual-controls`,"aria-label":`Residual explorer controls`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Choose one output`}),(0,E.jsx)(`h2`,{children:`Trace controls`}),(0,E.jsx)(`p`,{children:`Move from a clipped boundary field to the five-position center field.`}),(0,E.jsx)(`div`,{className:`residual-output-buttons`,children:i.output.map((n,r)=>(0,E.jsxs)(`button`,{"aria-label":`Select residual output ${r}`,className:r===e?`residual-output-button residual-output-button--active`:`residual-output-button`,type:`button`,onClick:()=>t(r),children:[(0,E.jsxs)(`small`,{children:[`y[`,r,`]`]}),(0,E.jsx)(`strong`,{children:Hu(n)})]},r))}),(0,E.jsxs)(`label`,{className:`residual-skip-control`,children:[(0,E.jsx)(`input`,{type:`checkbox`,checked:n,onChange:e=>r(e.target.checked)}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`strong`,{children:`Include identity skip`}),(0,E.jsx)(`small`,{children:`Add x[i] directly to the main path.`})]})]}),(0,E.jsxs)(`div`,{className:`button-grid`,children:[(0,E.jsx)(`button`,{type:`button`,disabled:e===0,onClick:()=>t(e=>Math.max(0,e-1)),children:`Previous output`}),(0,E.jsx)(`button`,{type:`button`,disabled:e===i.output.length-1,onClick:()=>t(e=>Math.min(i.output.length-1,e+1)),children:`Next output`}),(0,E.jsx)(`button`,{type:`button`,onClick:u,children:`Reset trace`})]}),(0,E.jsxs)(`div`,{className:`residual-note`,children:[(0,E.jsx)(`span`,{children:`What scales next?`}),(0,E.jsx)(`p`,{children:`More layers widen the main path's field. Projection skips handle shape changes, but must still land on a tensor compatible with the addition.`})]})]})]})}var Gu=[1,-1,1,-1],Ku=[-1,-1,1,-1],qu=[0,1,2,3];function J(e){return Math.abs(e)<1e-12?0:e}function Ju(e,t){if(e.length<2||e.some(e=>e!==-1&&e!==1))throw Error(`${t} must contain at least two bipolar values (-1 or +1).`)}function Yu(e){let t=e.length;return e.map((n,r)=>e.map((e,i)=>r===i?0:n*e/t))}function Xu(e,t){let n=0;for(let r=0;r<e.length;r+=1)for(let i=0;i<e.length;i+=1)n+=t[r][i]*e[r]*e[i];return J(-.5*n)}function Zu(e,t){return e.reduce((e,n,r)=>e+n*t[r],0)/e.length}function Qu(e,t){return e.filter((e,n)=>e!==t[n]).length}function $u(e=Gu,t=Ku,n=qu){if(Ju(e,`storedPattern`),Ju(t,`corruptedState`),e.length!==t.length||n.length!==e.length||new Set(n).size!==n.length||n.some(t=>!Number.isInteger(t)||t<0||t>=e.length))throw Error(`NN20 V1 needs equal-sized states and one permutation of every neuron index.`);let r=[...e],i=[...t],a=Yu(r),o=Xu(i,a),s=Zu(r,i),c=[],l=[...i];n.forEach((e,t)=>{let n=[...l],i=n.map((t,n)=>{let r=a[e][n];return{sourceIndex:n,weight:r,sourceState:t,contribution:J(r*t)}}),o=J(i.reduce((e,t)=>e+t.contribution,0)),s=n[e],u=o>0?1:o<0?-1:s;l=[...n],l[e]=u,c.push({step:t,neuronIndex:e,stateBefore:n,incoming:i,localField:o,previousState:s,nextState:u,changed:u!==s,stateAfter:[...l],energyBefore:Xu(n,a),energyAfter:Xu(l,a),overlapBefore:Zu(r,n),overlapAfter:Zu(r,l)})});let u=Xu(l,a),d=Zu(r,l),f=Qu(r,l);return{storedPattern:r,normalization:r.length,weights:a,corruptedState:i,updateOrder:[...n],initialEnergy:o,initialOverlap:s,initialHammingDistance:Qu(r,i),updates:c,finalState:[...l],finalEnergy:u,finalOverlap:d,finalHammingDistance:f,converged:f===0&&c.every(e=>e.energyAfter<=e.energyBefore+1e-12)}}function ed(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):e.toFixed(2).replace(/0+$/,``).replace(/\.$/,``)}function td(e){return`[${e.map(e=>e>0?`+1`:`-1`).join(`, `)}]`}var nd=[{eyebrow:`0. Store`,title:`Hebbian weights`},{eyebrow:`1. Cue`,title:`One flipped bit`},{eyebrow:`2. Recall`,title:`Update neuron 0`},{eyebrow:`3. Recall`,title:`Update neuron 1`},{eyebrow:`4. Recall`,title:`Update neuron 2`},{eyebrow:`5. Recall`,title:`Update neuron 3`}];function rd(){let e=(0,l.useMemo)(()=>$u(),[]),[t,n]=(0,l.useState)(0),r=Math.max(t-1,0),i=r>0?e.updates[r-1]:null,a=t===0?e.storedPattern:i?.stateAfter??e.corruptedState,o=t===0?e.finalEnergy:i?.energyAfter??e.initialEnergy,s=t===0?1:i?.overlapAfter??e.initialOverlap;return(0,E.jsxs)(`main`,{className:`workspace workspace--hopfield`,children:[(0,E.jsxs)(`section`,{className:`hopfield-stage`,"aria-label":`Hopfield associative memory trace`,children:[(0,E.jsxs)(`div`,{className:`hopfield-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN20 - a remembered pattern becomes an attractor`}),(0,E.jsx)(`h2`,{children:`Restore one flipped bit with four connected neurons`}),(0,E.jsx)(`p`,{children:`Store a bipolar pattern in symmetric weights, present a damaged cue, and audit every asynchronous update as energy moves downhill.`})]}),(0,E.jsx)(`div`,{className:`hopfield-chip`,children:`4 neurons - 1 memory`})]}),(0,E.jsxs)(`section`,{className:`hopfield-store-panel`,"aria-label":`Hopfield Hebbian storage rule`,children:[(0,E.jsxs)(`div`,{className:`hopfield-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{children:`Outer product, then erase self-connections`}),(0,E.jsx)(`h2`,{children:`Turn the saved pattern into weights`})]}),(0,E.jsx)(`code`,{children:`w_ij = p_i p_j / 4, w_ii = 0`})]}),(0,E.jsxs)(`div`,{className:`hopfield-pattern-row`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`stored pattern p`}),(0,E.jsx)(`strong`,{children:td(e.storedPattern)})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`normalization`}),(0,E.jsxs)(`strong`,{children:[`divide by `,e.normalization]})]}),(0,E.jsx)(`span`,{"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`diagonal`}),(0,E.jsx)(`strong`,{children:`set to 0`})]})]}),(0,E.jsxs)(`div`,{className:`hopfield-matrix`,role:`table`,"aria-label":`Hopfield learned weight matrix`,children:[(0,E.jsx)(`div`,{className:`hopfield-matrix__corner`}),e.storedPattern.map((e,t)=>(0,E.jsxs)(`b`,{children:[`from `,t]},`column-${t}`)),e.weights.map((e,t)=>(0,E.jsxs)(`div`,{className:`hopfield-matrix__row`,role:`row`,children:[(0,E.jsxs)(`b`,{children:[`to `,t]}),e.map((e,n)=>(0,E.jsx)(`code`,{className:t===n?`hopfield-weight hopfield-weight--diagonal`:`hopfield-weight`,children:ed(e)},`${t}-${n}`))]},`row-${t}`))]}),(0,E.jsx)(`p`,{className:`hopfield-note`,children:`Symmetry makes the energy score valid. A zero diagonal keeps each neuron from voting for itself.`})]}),(0,E.jsxs)(`section`,{className:`hopfield-recall-panel`,"aria-label":`Hopfield asynchronous recall trace`,children:[(0,E.jsxs)(`div`,{className:`hopfield-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{children:`Use the newest state immediately`}),(0,E.jsx)(`h2`,{children:`Recall one neuron at a time`})]}),(0,E.jsx)(`code`,{children:`state_i = sign(sum_j w_ij state_j)`})]}),(0,E.jsxs)(`div`,{className:`hopfield-recall-lane`,children:[(0,E.jsxs)(`div`,{className:`hopfield-state`,children:[(0,E.jsx)(`small`,{children:`damaged cue`}),(0,E.jsx)(`strong`,{children:td(e.corruptedState)}),(0,E.jsxs)(`span`,{children:[`distance `,e.initialHammingDistance]})]}),e.updates.map((e,t)=>(0,E.jsxs)(`div`,{className:r>t?`hopfield-update hopfield-update--visible`:`hopfield-update`,children:[(0,E.jsxs)(`small`,{children:[`update `,e.neuronIndex]}),(0,E.jsx)(`strong`,{children:r>t?td(e.stateAfter):`?`}),(0,E.jsx)(`span`,{children:r>t?`field ${ed(e.localField)}`:`advance to reveal`})]},e.step))]}),(0,E.jsxs)(`div`,{className:`hopfield-audit-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`visible state`}),(0,E.jsx)(`strong`,{children:td(a)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`normalized overlap`}),(0,E.jsx)(`strong`,{children:ed(s)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`Hopfield energy`}),(0,E.jsx)(`strong`,{children:ed(o)})]})]}),i===null?(0,E.jsx)(`div`,{className:`hopfield-contribution-panel`,children:(0,E.jsx)(`p`,{children:t===0?`The stored pattern is already a low-energy fixed point.`:`The cue matches three of four saved bits. Update neuron 0 first.`})}):(0,E.jsxs)(`div`,{className:`hopfield-contribution-panel`,"aria-label":`Hopfield active neuron calculation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`active neuron`}),(0,E.jsx)(`strong`,{children:i.neuronIndex})]}),(0,E.jsx)(`div`,{className:`hopfield-contributions`,children:i.incoming.map(e=>(0,E.jsxs)(`code`,{children:[ed(e.weight),` x `,e.sourceState>0?`+1`:`-1`,` = `,ed(e.contribution)]},e.sourceIndex))}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`local field -> next state`}),(0,E.jsxs)(`strong`,{children:[ed(i.localField),` -> `,i.nextState>0?`+1`:`-1`]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`energy before -> after`}),(0,E.jsxs)(`strong`,{children:[ed(i.energyBefore),` -> `,ed(i.energyAfter)]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`overlap before -> after`}),(0,E.jsxs)(`strong`,{children:[ed(i.overlapBefore),` -> `,ed(i.overlapAfter)]})]})]})]})]}),(0,E.jsxs)(`aside`,{className:`hopfield-controls`,"aria-label":`Hopfield phase controls`,children:[(0,E.jsx)(`p`,{children:`Associative recall`}),(0,E.jsx)(`h2`,{children:`Advance the memory`}),(0,E.jsx)(`p`,{children:`The first recall step repairs the flipped bit. The other steps prove the recovered pattern is stable under a complete deterministic sweep.`}),(0,E.jsx)(`div`,{className:`hopfield-phase-buttons`,children:nd.map((e,r)=>(0,E.jsxs)(`button`,{"aria-pressed":t===r,type:`button`,onClick:()=>n(r),children:[(0,E.jsx)(`span`,{children:e.eyebrow}),(0,E.jsx)(`strong`,{children:e.title})]},e.title))}),(0,E.jsxs)(`div`,{className:`hopfield-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected state`}),(0,E.jsx)(`strong`,{children:nd[t].title}),(0,E.jsxs)(`span`,{children:[`energy = `,ed(o)]}),(0,E.jsxs)(`span`,{children:[`overlap = `,ed(s)]}),t===nd.length-1?(0,E.jsx)(`b`,{children:`fixed point recovered`}):null]})]})]})}var id=[1,2,-1],ad=[{source:0,target:1},{source:1,target:2}],od={messageWeight:.5,selfWeight:.25,bias:-.5};function sd(e){return Math.abs(e)<1e-12?0:e}function cd(e=id,t=ad,n=od){let r=[...e,n.messageWeight,n.selfWeight,n.bias];if(e.length<2||!r.every(Number.isFinite)||t.length<1||t.some(t=>!Number.isInteger(t.source)||!Number.isInteger(t.target)||t.source<0||t.target<0||t.source>=e.length||t.target>=e.length||t.source===t.target))throw Error(`NN21 V1 needs finite node features and valid non-self undirected edges.`);let i=t.map(e=>`${Math.min(e.source,e.target)}-${Math.max(e.source,e.target)}`);if(new Set(i).size!==i.length)throw Error(`NN21 V1 needs unique undirected edges.`);let a=t.flatMap(e=>[{source:e.source,target:e.target},{source:e.target,target:e.source}]).map(({source:t,target:r})=>{let i=e[t];return{source:t,target:r,sourceFeature:i,messageWeight:n.messageWeight,message:sd(n.messageWeight*i)}}).sort((e,t)=>e.target-t.target||e.source-t.source),o=e.map((e,t)=>{let r=a.filter(e=>e.target===t),i=sd(r.reduce((e,t)=>e+t.message,0)),o=sd(n.selfWeight*e),s=sd(o+i+n.bias);return{node:t,oldFeature:e,incoming:r,aggregate:i,selfContribution:o,bias:n.bias,preactivation:s,outputFeature:Math.max(0,s)}});return{nodeFeatures:[...e],edges:t.map(e=>({...e})),parameters:{...n},directedMessages:a,nodeUpdates:o,outputFeatures:o.map(e=>e.outputFeature)}}function ld(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):e.toFixed(2).replace(/0+$/,``).replace(/\.$/,``)}var ud=[`Graph`,`Messages`,`Aggregate`,`Update`];function dd(){let e=(0,l.useMemo)(()=>cd(),[]),[t,n]=(0,l.useState)(`Graph`),[r,i]=(0,l.useState)(1),a=e.nodeUpdates[r],o=t!==`Graph`,s=t===`Aggregate`||t===`Update`,c=t===`Update`;return(0,E.jsxs)(`main`,{className:`workspace workspace--message-passing`,children:[(0,E.jsxs)(`section`,{className:`message-stage`,"aria-label":`Tiny graph message-passing trace`,children:[(0,E.jsxs)(`div`,{className:`message-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN21 - neighbors send, nodes collect, one round updates`}),(0,E.jsx)(`h2`,{children:`Pass scalar messages across a three-node path`}),(0,E.jsx)(`p`,{children:`Expand two undirected edges into four directed messages, sum each inbox, and update all nodes from the same saved feature snapshot.`})]}),(0,E.jsx)(`div`,{className:`message-chip`,children:`3 nodes - 2 edges`})]}),(0,E.jsxs)(`section`,{className:`message-graph-panel`,"aria-label":`Tiny graph and directed messages`,children:[(0,E.jsxs)(`div`,{className:`message-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{children:`Synchronous round`}),(0,E.jsx)(`h2`,{children:`Original features stay fixed while messages travel`})]}),(0,E.jsx)(`code`,{children:`m(source -> target) = 0.5 x source`})]}),(0,E.jsxs)(`div`,{className:`message-graph`,children:[e.nodeFeatures.map((t,n)=>(0,E.jsxs)(`button`,{className:r===n?`message-node message-node--selected`:`message-node`,type:`button`,onClick:()=>i(n),children:[(0,E.jsxs)(`small`,{children:[`node `,n]}),(0,E.jsx)(`strong`,{children:ld(c?e.outputFeatures[n]:t)}),(0,E.jsx)(`span`,{children:c?`new feature`:`old feature`})]},n)),(0,E.jsx)(`div`,{className:`message-edge message-edge--left`,children:`0 <-> 1`}),(0,E.jsx)(`div`,{className:`message-edge message-edge--right`,children:`1 <-> 2`})]}),(0,E.jsx)(`div`,{className:`message-ledger`,children:e.directedMessages.map(e=>(0,E.jsxs)(`div`,{className:o&&e.target===r?`message-card message-card--active`:`message-card`,children:[(0,E.jsxs)(`small`,{children:[e.source,` -> `,e.target]}),(0,E.jsxs)(`code`,{children:[`0.5 x `,ld(e.sourceFeature)]}),(0,E.jsx)(`strong`,{children:o?ld(e.message):`?`})]},`${e.source}-${e.target}`))})]}),(0,E.jsxs)(`section`,{className:`message-update-panel`,"aria-label":`Selected graph node update`,children:[(0,E.jsxs)(`div`,{className:`message-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{children:[`Selected node `,r]}),(0,E.jsx)(`h2`,{children:`Open its inbox and update equation`})]}),(0,E.jsx)(`code`,{children:`ReLU(0.25 x self + sum(messages) - 0.5)`})]}),(0,E.jsxs)(`div`,{className:`message-inbox`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`incoming messages`}),(0,E.jsx)(`strong`,{children:o?a.incoming.map(e=>ld(e.message)).join(` + `):`hidden`})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`sum aggregate`}),(0,E.jsx)(`strong`,{children:s?ld(a.aggregate):`?`})]})]}),(0,E.jsxs)(`div`,{className:`message-equation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`self route`}),(0,E.jsxs)(`code`,{children:[`0.25 x `,ld(a.oldFeature)]}),(0,E.jsx)(`strong`,{children:s?ld(a.selfContribution):`?`})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`neighbor route`}),(0,E.jsx)(`code`,{children:`sum inbox`}),(0,E.jsx)(`strong`,{children:s?ld(a.aggregate):`?`})]}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`bias`}),(0,E.jsx)(`code`,{children:`-0.5`}),(0,E.jsx)(`strong`,{children:s?`-0.5`:`?`})]}),(0,E.jsx)(`span`,{children:`=`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`preactivation`}),(0,E.jsx)(`code`,{children:`before ReLU`}),(0,E.jsx)(`strong`,{children:s?ld(a.preactivation):`?`})]}),(0,E.jsx)(`span`,{children:`->`}),(0,E.jsxs)(`div`,{className:`message-output`,children:[(0,E.jsx)(`small`,{children:`new feature`}),(0,E.jsx)(`code`,{children:`ReLU`}),(0,E.jsx)(`strong`,{children:c?ld(a.outputFeature):`?`})]})]}),(0,E.jsx)(`p`,{className:`message-sync-note`,children:"All four messages use the original features `[1, 2, -1]`. No node reads another node's new output during this round."})]})]}),(0,E.jsxs)(`aside`,{className:`message-controls`,"aria-label":`Message-passing phase controls`,children:[(0,E.jsx)(`p`,{children:`One graph round`}),(0,E.jsx)(`h2`,{children:`Reveal the pipeline`}),(0,E.jsx)(`p`,{children:`Select any node, then expose directed messages, its order-invariant sum, and the shared update rule.`}),(0,E.jsx)(`div`,{className:`message-phase-buttons`,children:ud.map((e,r)=>(0,E.jsxs)(`button`,{"aria-pressed":t===e,type:`button`,onClick:()=>n(e),children:[(0,E.jsxs)(`span`,{children:[r,`. Phase`]}),(0,E.jsx)(`strong`,{children:e})]},e))}),(0,E.jsxs)(`div`,{className:`message-selected-summary`,children:[(0,E.jsx)(`small`,{children:`selected node`}),(0,E.jsx)(`strong`,{children:r}),(0,E.jsxs)(`span`,{children:[`neighbors = `,a.incoming.map(e=>e.source).join(`, `)]}),(0,E.jsxs)(`span`,{children:[`output = `,c?ld(a.outputFeature):`?`]}),c?(0,E.jsx)(`b`,{children:`round complete`}):null]})]})]})}var fd=[[0,1],[0,1,2],[1,2]],pd=[1,2,-1];function md(e=pd,t=fd){if(e.length<2||!e.every(Number.isFinite)||t.length!==e.length)throw Error(`NN22 V1 needs finite features and one neighborhood per node.`);t.forEach((t,n)=>{if(t.length<1||new Set(t).size!==t.length||!t.includes(n)||t.some(t=>!Number.isInteger(t)||t<0||t>=e.length))throw Error(`NN22 V1 neighborhoods must be unique valid indices and include self-loops.`)});for(let e=0;e<t.length;e+=1)for(let n of t[e])if(!t[n].includes(e))throw Error(`NN22 V1 neighborhoods must be symmetric.`);let n=t.map(e=>e.length),r=t.map((t,r)=>{let i=t.map(t=>{let i=1/Math.sqrt(n[r]*n[t]);return{source:t,sourceFeature:e[t],sourceDegree:n[t],targetDegree:n[r],coefficient:i,contribution:i*e[t]}}),a=i.reduce((e,t)=>e+t.contribution,0);return{target:r,rows:i,preactivation:a,output:Math.max(0,a)}}),i=t.map((t,n)=>{let r=t.map(t=>e[t]),i=Math.max(...r),a=r.map(e=>Math.exp(e-i)),o=a.reduce((e,t)=>e+t,0),s=t.map((t,n)=>{let s=a[n]/o;return{source:t,sourceFeature:e[t],score:r[n],shiftedScore:r[n]-i,exponential:a[n],attentionWeight:s,contribution:s*e[t]}}),c=s.reduce((e,t)=>e+t.contribution,0);return{target:n,rows:s,maximumScore:i,denominator:o,preactivation:c,output:Math.max(0,c)}});return{features:[...e],neighborhoods:t.map(e=>[...e]),degrees:n,gcn:r,gat:i,gcnOutputs:r.map(e=>e.output),gatOutputs:i.map(e=>e.output)}}function hd(e){return Math.abs(e)<1e-12?`0`:Number.isInteger(e)?String(e):e.toFixed(6).replace(/0+$/,``)}function gd(){let e=(0,l.useMemo)(()=>md(),[]),[t,n]=(0,l.useState)(`gcn`),[r,i]=(0,l.useState)(1),a=e.gcn[r],o=e.gat[r];return(0,E.jsxs)(`main`,{className:`workspace workspace--graph-neighborhood`,children:[(0,E.jsxs)(`section`,{className:`graph-neighborhood-stage`,"aria-label":`Graph convolution and attention trace`,children:[(0,E.jsxs)(`div`,{className:`graph-neighborhood-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN22 - same neighborhood, two weighting rules`}),(0,E.jsx)(`h2`,{children:`Compare graph convolution with graph attention`}),(0,E.jsx)(`p`,{children:`Add self-loops to one three-node path, then inspect fixed degree normalization beside learned softmax attention.`})]}),(0,E.jsx)(`div`,{className:`graph-neighborhood-chip`,children:`GCN vs GAT`})]}),(0,E.jsxs)(`section`,{className:`graph-neighborhood-map`,"aria-label":`Graph neighborhood selector`,children:[(0,E.jsxs)(`div`,{className:`graph-neighborhood-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{children:`Original scalar features`}),(0,E.jsx)(`h2`,{children:`Select a target neighborhood`})]}),(0,E.jsx)(`code`,{children:`0(1) <-> 1(2) <-> 2(-1), plus self-loops`})]}),(0,E.jsx)(`div`,{className:`graph-targets`,children:e.features.map((t,n)=>(0,E.jsxs)(`button`,{"aria-pressed":r===n,type:`button`,onClick:()=>i(n),children:[(0,E.jsxs)(`small`,{children:[`node `,n]}),(0,E.jsx)(`strong`,{children:hd(t)}),(0,E.jsxs)(`span`,{children:[`degree `,e.degrees[n]]})]},n))}),(0,E.jsxs)(`p`,{children:[`Target `,r,` reads sources [`,e.neighborhoods[r].join(`, `),`]. Both models use exactly this same inbox.`]})]}),t===`gcn`?(0,E.jsxs)(`section`,{className:`graph-model-panel`,"aria-label":`Graph convolution calculation`,children:[(0,E.jsxs)(`div`,{className:`graph-neighborhood-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{children:`Fixed structural weights`}),(0,E.jsx)(`h2`,{children:`Normalize by both endpoint degrees`})]}),(0,E.jsx)(`code`,{children:`coefficient = 1 / sqrt(d_target x d_source)`})]}),(0,E.jsx)(`div`,{className:`graph-row-grid`,children:a.rows.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`source `,e.source]}),(0,E.jsxs)(`code`,{children:[`1 / sqrt(`,e.targetDegree,` x `,e.sourceDegree,`)`]}),(0,E.jsx)(`strong`,{children:hd(e.coefficient)}),(0,E.jsxs)(`span`,{children:[`x feature `,hd(e.sourceFeature)]}),(0,E.jsxs)(`b`,{children:[`= `,hd(e.contribution)]})]},e.source))}),(0,E.jsxs)(`div`,{className:`graph-result`,children:[(0,E.jsx)(`span`,{children:`sum contributions`}),(0,E.jsxs)(`strong`,{children:[a.rows.map(e=>hd(e.contribution)).join(` + `),` = `,hd(a.preactivation)]}),(0,E.jsxs)(`b`,{children:[`ReLU -> `,hd(a.output)]})]})]}):(0,E.jsxs)(`section`,{className:`graph-model-panel`,"aria-label":`Graph attention calculation`,children:[(0,E.jsxs)(`div`,{className:`graph-neighborhood-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{children:`Data-dependent weights`}),(0,E.jsx)(`h2`,{children:`Softmax the source scores inside this inbox`})]}),(0,E.jsx)(`code`,{children:`score = source feature; alpha = stable softmax(score)`})]}),(0,E.jsxs)(`div`,{className:`graph-softmax-summary`,children:[(0,E.jsxs)(`span`,{children:[`row max = `,hd(o.maximumScore)]}),(0,E.jsxs)(`span`,{children:[`denominator = `,hd(o.denominator)]}),(0,E.jsxs)(`strong`,{children:[`weights sum = `,hd(o.rows.reduce((e,t)=>e+t.attentionWeight,0))]})]}),(0,E.jsx)(`div`,{className:`graph-row-grid`,children:o.rows.map(e=>(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`source `,e.source]}),(0,E.jsxs)(`code`,{children:[`score `,hd(e.score),` - max `,hd(o.maximumScore),` = `,hd(e.shiftedScore)]}),(0,E.jsxs)(`span`,{children:[`exp = `,hd(e.exponential)]}),(0,E.jsxs)(`strong`,{children:[`alpha = `,hd(e.attentionWeight)]}),(0,E.jsxs)(`b`,{children:[`x `,hd(e.sourceFeature),` = `,hd(e.contribution)]})]},e.source))}),(0,E.jsxs)(`div`,{className:`graph-result`,children:[(0,E.jsx)(`span`,{children:`weighted sum`}),(0,E.jsxs)(`strong`,{children:[o.rows.map(e=>hd(e.contribution)).join(` + `),` = `,hd(o.preactivation)]}),(0,E.jsxs)(`b`,{children:[`ReLU -> `,hd(o.output)]})]})]}),(0,E.jsxs)(`section`,{className:`graph-output-panel`,"aria-label":`Graph model output comparison`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`GCN outputs`}),(0,E.jsxs)(`strong`,{children:[`[`,e.gcnOutputs.map(hd).join(`, `),`]`]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`GAT outputs`}),(0,E.jsxs)(`strong`,{children:[`[`,e.gatOutputs.map(hd).join(`, `),`]`]})]}),(0,E.jsx)(`p`,{children:`GCN weights depend only on graph degrees. GAT weights change with the node features, even though the edges are unchanged.`})]})]}),(0,E.jsxs)(`aside`,{className:`graph-neighborhood-controls`,"aria-label":`Graph model controls`,children:[(0,E.jsx)(`p`,{children:`Neighborhood model`}),(0,E.jsx)(`h2`,{children:`Switch the weighting rule`}),(0,E.jsx)(`p`,{children:`Keep the target and graph fixed while changing how its inbox is weighted.`}),(0,E.jsxs)(`button`,{"aria-pressed":t===`gcn`,type:`button`,onClick:()=>n(`gcn`),children:[(0,E.jsx)(`span`,{children:`Degree rule`}),(0,E.jsx)(`strong`,{children:`Graph convolution`})]}),(0,E.jsxs)(`button`,{"aria-pressed":t===`gat`,type:`button`,onClick:()=>n(`gat`),children:[(0,E.jsx)(`span`,{children:`Softmax rule`}),(0,E.jsx)(`strong`,{children:`Graph attention`})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`selected target`}),(0,E.jsx)(`strong`,{children:r}),(0,E.jsx)(`span`,{children:t===`gcn`?`structural coefficients`:`feature-dependent attention`})]})]})]})}function _d(){let[e,t]=(0,l.useState)(`hopfield`);return(0,E.jsxs)(`div`,{className:`structured-workbench`,children:[(0,E.jsxs)(`nav`,{className:`structured-lab-switch`,"aria-label":`Structured and memory learning lab`,children:[(0,E.jsx)(`button`,{"aria-pressed":e===`hopfield`,type:`button`,onClick:()=>t(`hopfield`),children:`Hopfield memory`}),(0,E.jsx)(`button`,{"aria-pressed":e===`message`,type:`button`,onClick:()=>t(`message`),children:`Message passing`}),(0,E.jsx)(`button`,{"aria-pressed":e===`graph-models`,type:`button`,onClick:()=>t(`graph-models`),children:`GCN vs GAT`})]}),e===`hopfield`?(0,E.jsx)(rd,{}):e===`message`?(0,E.jsx)(dd,{}):(0,E.jsx)(gd,{})]})}var vd=4,yd=8,bd=64,xd=1e6,Sd=[{id:`outer-grid`,title:`Column + row`,summary:`Both inputs expand along one axis.`,left:{shape:[2,1],values:[1,2]},right:{shape:[1,3],values:[10,20,30]},upstream:{shape:[2,3],values:[1,2,3,4,5,6]}},{id:`row-over-batch`,title:`Matrix + rank-one row`,summary:`Right alignment turns [3] into [1, 3].`,left:{shape:[2,3],values:[1,2,3,4,5,6]},right:{shape:[3],values:[10,20,30]},upstream:{shape:[2,3],values:[1,1,1,1,1,1]}},{id:`scalar-over-matrix`,title:`Scalar + matrix`,summary:`A rank-zero value reaches every output cell.`,left:{shape:[],values:[2]},right:{shape:[2,2],values:[1,2,3,4]},upstream:{shape:[2,2],values:[1,-1,2,-2]}},{id:`incompatible-tail`,title:`Mismatch`,summary:`Trailing dimensions 3 and 2 cannot align.`,left:{shape:[2,3],values:[1,2,3,4,5,6]},right:{shape:[2],values:[10,20]},upstream:null}];function Y(e){return e.reduce((e,t)=>e*t,1)}function Cd(e,t){if(typeof e!=`object`||!e||!Array.isArray(e.shape)||!Array.isArray(e.values))throw Error(`${t} must contain shape and values arrays`);if(e.shape.length>vd)throw Error(`${t} shape must contain at most ${vd} dimensions`);e.shape.forEach(e=>{if(!Number.isInteger(e)||e<=0||e>yd)throw Error(`${t} dimensions must be positive integers up to ${yd}`)});let n=Y(e.shape);if(n>bd||e.values.length!==n)throw Error(`${t} values must match its bounded shape`);if(!e.values.every(e=>Number.isFinite(e)&&Math.abs(e)<=xd))throw Error(`${t} values must be finite and bounded`)}function wd(e,t){if(!Number.isFinite(e))throw Error(`${t} must remain finite`);return e}function Td(e){let t=Array(e.length).fill(0),n=1;for(let r=e.length-1;r>=0;--r)t[r]=n,n*=e[r];return t}function Ed(e,t){return Td(t).map(t=>{let n=Math.floor(e/t);return e%=t,n})}function Dd(e,t){return e.reduce((e,n,r)=>e+n*Td(t)[r],0)}function Od(e,t){let n=Math.max(e.length,t.length);return[[...Array(n-e.length).fill(1),...e],[...Array(n-t.length).fill(1),...t]]}function kd(e,t,n){let r=0;return n.forEach(n=>{let i=wd(e[n.leftFlatIndex]+t[n.rightFlatIndex],`broadcast score output`),a=wd(n.upstream*i,`broadcast score contribution`);r=wd(r+a,`broadcast score`)}),r}function Ad(e,t,n,r=1e-5){if(Cd(e,`left tensor`),Cd(t,`right tensor`),!Number.isFinite(r)||r<1e-12||r>1)throw Error(`finite-difference epsilon must be finite and in [1e-12, 1]`);let[i,a]=Od(e.shape,t.shape),o=[];for(let n=0;n<i.length;n+=1){let r=i[n],s=a[n];if(r!==s&&r!==1&&s!==1)return{compatible:!1,left:e,right:t,upstream:null,paddedLeftShape:i,paddedRightShape:a,mismatchAxis:n,leftDimension:r,rightDimension:s,error:`axis ${n}: dimensions ${r} and ${s} are incompatible`};o.push(Math.max(r,s))}if(n===null)throw Error(`compatible shapes require an upstream tensor`);if(Cd(n,`upstream tensor`),n.shape.length!==o.length||n.shape.some((e,t)=>e!==o[t]))throw Error(`upstream shape must equal output shape [${o.join(`, `)}]`);let s=o.length,c=s-e.shape.length,l=s-t.shape.length,u=[];for(let r=0;r<Y(o);r+=1){let s=Ed(r,o),d=s.map((e,t)=>i[t]===1?0:e),f=s.map((e,t)=>a[t]===1?0:e),p=d.slice(c),m=f.slice(l),h=Dd(p,e.shape),g=Dd(m,t.shape),_=e.values[h],v=t.values[g],y=wd(_+v,`broadcast output`);u.push({outputIndex:s,outputFlatIndex:r,leftIndex:p,leftFlatIndex:h,rightIndex:m,rightFlatIndex:g,leftValue:_,rightValue:v,outputValue:y,upstream:n.values[r]})}let d=Array(e.values.length).fill(0),f=Array(t.values.length).fill(0);u.forEach(e=>{d[e.leftFlatIndex]=wd(d[e.leftFlatIndex]+e.upstream,`left broadcast gradient`),f[e.rightFlatIndex]=wd(f[e.rightFlatIndex]+e.upstream,`right broadcast gradient`)});let p=e.values.map((n,i)=>{let a=[...e.values],o=[...e.values];return a[i]+=r,o[i]-=r,wd((kd(a,t.values,u)-kd(o,t.values,u))/(2*r),`left finite-difference gradient`)}),m=t.values.map((n,i)=>{let a=[...t.values],o=[...t.values];return a[i]+=r,o[i]-=r,wd((kd(e.values,a,u)-kd(e.values,o,u))/(2*r),`right finite-difference gradient`)}),h=[...d.map((e,t)=>Math.abs(e-p[t])),...f.map((e,t)=>Math.abs(e-m[t]))],g=wd(Math.max(...h,0),`gradient error`);return{compatible:!0,left:e,right:t,upstream:n,paddedLeftShape:i,paddedRightShape:a,outputShape:o,leftExpandedAxes:i.flatMap((e,t)=>e===1&&o[t]>1?[t]:[]),rightExpandedAxes:a.flatMap((e,t)=>e===1&&o[t]>1?[t]:[]),outputValues:u.map(e=>e.outputValue),mappings:u,leftGradient:d,rightGradient:f,finiteDifferenceLeftGradient:p,finiteDifferenceRightGradient:m,maxGradientAbsoluteError:g}}function jd(e=`outer-grid`){let t=Sd.find(t=>t.id===e);if(t===void 0)throw Error(`unknown tensor broadcasting scenario: ${e}`);return{id:t.id,title:t.title,summary:t.summary,...Ad(t.left,t.right,t.upstream)}}function Md(e,t=6){return Math.abs(e)<1e-12?`0`:Math.abs(e)<1e-4||Math.abs(e)>=1e3?e.toExponential(3):Number(e.toFixed(t)).toString()}function Nd(e){return e.length===0?`[] scalar`:`[${e.join(`, `)}]`}function Pd(e){return e.length===0?`[]`:`[${e.join(`, `)}]`}function Fd(e){return`[${e.map(e=>Md(e)).join(`, `)}]`}function Id({trace:e}){let t=e.compatible?-1:e.mismatchAxis;return(0,E.jsxs)(`section`,{className:`tensor-shape-panel`,"aria-label":`Right aligned tensor shapes`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 1 / line up the tail`}),(0,E.jsx)(`h2`,{children:`Compare dimensions from the right`})]}),(0,E.jsx)(`span`,{children:`equal or one`})]}),(0,E.jsxs)(`div`,{className:`tensor-shape-equation`,children:[(0,E.jsx)(`code`,{children:Nd(e.left.shape)}),(0,E.jsx)(`span`,{children:`+`}),(0,E.jsx)(`code`,{children:Nd(e.right.shape)}),(0,E.jsx)(`span`,{children:`→`}),(0,E.jsx)(`strong`,{children:e.compatible?Nd(e.outputShape):`shape error`})]}),(0,E.jsx)(`div`,{className:`tensor-axis-grid`,children:e.paddedLeftShape.map((n,r)=>{let i=e.paddedRightShape[r],a=n===i||n===1||i===1;return(0,E.jsxs)(`div`,{className:r===t?`is-mismatch`:``,children:[(0,E.jsxs)(`small`,{children:[`axis `,r]}),(0,E.jsxs)(`code`,{children:[n,` ↔ `,i]}),(0,E.jsx)(`strong`,{children:a?Math.max(n,i):`stop`}),(0,E.jsx)(`span`,{children:n===i?`same`:a?`expand the 1`:`neither is 1`})]},r)})})]})}function Ld({trace:e}){return(0,E.jsxs)(`section`,{className:`tensor-gradient-panel`,"aria-label":`Broadcast gradient reduction`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 4 / reverse the reuse`}),(0,E.jsx)(`h2`,{children:`Copied routes add back together`})]}),(0,E.jsx)(`span`,{children:`sum expanded axes`})]}),(0,E.jsxs)(`div`,{className:`tensor-gradient-grid`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`upstream / output shape `,Nd(e.outputShape)]}),(0,E.jsx)(`code`,{children:Fd(e.upstream.values)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`left gradient / original shape `,Nd(e.left.shape)]}),(0,E.jsx)(`code`,{children:Fd(e.leftGradient)}),(0,E.jsxs)(`span`,{children:[`reduce axes `,e.leftExpandedAxes.length?e.leftExpandedAxes.join(`, `):`none`]})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`small`,{children:[`right gradient / original shape `,Nd(e.right.shape)]}),(0,E.jsx)(`code`,{children:Fd(e.rightGradient)}),(0,E.jsxs)(`span`,{children:[`reduce axes `,e.rightExpandedAxes.length?e.rightExpandedAxes.join(`, `):`none`]})]})]}),(0,E.jsxs)(`div`,{className:`tensor-gradient-audit`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`finite-difference epsilon`}),(0,E.jsx)(`code`,{children:`1e-5`})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`left numerical`}),(0,E.jsx)(`code`,{children:Fd(e.finiteDifferenceLeftGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`right numerical`}),(0,E.jsx)(`code`,{children:Fd(e.finiteDifferenceRightGradient)})]}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`maximum absolute error`}),(0,E.jsx)(`code`,{children:Md(e.maxGradientAbsoluteError)})]})]})]})}function Rd(){let[e,t]=(0,l.useState)(`outer-grid`),[n,r]=(0,l.useState)(0),i=(0,l.useMemo)(()=>jd(e),[e]),a=i.compatible?i.mappings[Math.min(n,i.mappings.length-1)]:null,o=i.compatible?i.outputShape.at(-1)??1:1;function s(e){t(e),r(0)}return(0,E.jsxs)(`main`,{className:`workspace workspace--tensor-broadcasting`,children:[(0,E.jsxs)(`section`,{className:`tensor-broadcast-stage`,"aria-label":`Tensor shape and broadcasting visualizer`,children:[(0,E.jsxs)(`div`,{className:`tensor-broadcast-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`NN26 / tensor and autograd bridge`}),(0,E.jsx)(`h2`,{children:`Shape and broadcasting microscope`}),(0,E.jsx)(`p`,{children:`A broadcast does not invent new parameters. It reuses an input coordinate wherever an aligned dimension is one.`})]}),(0,E.jsx)(`div`,{className:`tensor-broadcast-chip`,children:`row-major`})]}),(0,E.jsx)(Id,{trace:i}),i.compatible?(0,E.jsxs)(E.Fragment,{children:[(0,E.jsxs)(`section`,{className:`tensor-output-panel`,"aria-label":`Broadcast output coordinate map`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 2 / reuse coordinates`}),(0,E.jsx)(`h2`,{children:`Open any output cell`})]}),(0,E.jsxs)(`span`,{children:[i.outputValues.length,` row-major cells`]})]}),(0,E.jsx)(`div`,{className:`tensor-output-grid`,style:{"--tensor-columns":o},children:i.mappings.map(e=>(0,E.jsxs)(`button`,{"aria-label":`Open output ${Pd(e.outputIndex)} value ${Md(e.outputValue)}`,"aria-pressed":e.outputFlatIndex===n,type:`button`,onClick:()=>r(e.outputFlatIndex),children:[(0,E.jsx)(`small`,{children:Pd(e.outputIndex)}),(0,E.jsx)(`strong`,{children:Md(e.outputValue)})]},e.outputFlatIndex))})]}),(0,E.jsxs)(`section`,{className:`tensor-mapping-panel`,"aria-label":`Selected broadcast index calculation`,children:[(0,E.jsxs)(`div`,{className:`panel-heading`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Step 3 / one hand calculation`}),(0,E.jsxs)(`h2`,{children:[`Output `,Pd(a.outputIndex)]})]}),(0,E.jsxs)(`span`,{children:[`flat slot `,a.outputFlatIndex]})]}),(0,E.jsxs)(`div`,{className:`tensor-mapping-equation`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`left source`}),(0,E.jsxs)(`code`,{children:[Pd(a.leftIndex),` → `,Md(a.leftValue)]}),(0,E.jsx)(`span`,{children:i.leftExpandedAxes.length?`axis ${i.leftExpandedAxes.join(`, `)} reuses this slot`:`no left expansion`})]}),(0,E.jsx)(`strong`,{children:`+`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`right source`}),(0,E.jsxs)(`code`,{children:[Pd(a.rightIndex),` → `,Md(a.rightValue)]}),(0,E.jsx)(`span`,{children:i.rightExpandedAxes.length?`axis ${i.rightExpandedAxes.join(`, `)} reuses this slot`:`no right expansion`})]}),(0,E.jsx)(`strong`,{children:`=`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`small`,{children:`output`}),(0,E.jsxs)(`code`,{children:[Pd(a.outputIndex),` → `,Md(a.outputValue)]}),(0,E.jsxs)(`span`,{children:[`upstream gradient `,Md(a.upstream)]})]})]})]}),(0,E.jsx)(Ld,{trace:i})]}):(0,E.jsxs)(`section`,{className:`tensor-mismatch-panel`,"aria-label":`Broadcast shape mismatch`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Stop before touching the buffers`}),(0,E.jsxs)(`h2`,{children:[`Axis `,i.mismatchAxis,` cannot broadcast`]}),(0,E.jsxs)(`code`,{children:[i.leftDimension,` is not `,i.rightDimension,`, and neither dimension is 1`]}),(0,E.jsxs)(`p`,{children:[i.error,`. A tensor library should reject this deterministically instead of recycling values or reading beyond a buffer.`]})]})]}),(0,E.jsxs)(`aside`,{className:`controls tensor-broadcast-controls`,"aria-label":`Tensor broadcasting scenarios`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Shape presets`}),(0,E.jsx)(`h2`,{children:`Change one alignment rule`}),(0,E.jsx)(`div`,{className:`tensor-scenario-buttons`,children:Sd.map(t=>(0,E.jsxs)(`button`,{"aria-pressed":t.id===e,type:`button`,onClick:()=>s(t.id),children:[(0,E.jsx)(`strong`,{children:t.title}),(0,E.jsxs)(`code`,{children:[Nd(t.left.shape),` + `,Nd(t.right.shape)]}),(0,E.jsx)(`span`,{children:t.summary})]},t.id))}),(0,E.jsxs)(`div`,{className:`tensor-mental-model`,children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`Keep this picture`}),(0,E.jsx)(`h2`,{children:`Forward reuses. Backward sums.`}),(0,E.jsx)(`p`,{children:`First align the tail. Then replace each compatible one with the other dimension. Every reused route contributes when gradients return.`})]})]})]})}var zd={input:2,target:1,weight:.5,bias:.1,learningRate:.1,activation:`linear`};function Bd(e,t,n){switch(n){case`linear`:return 1;case`sigmoid`:return t*(1-t);case`tanh`:return 1-t*t;case`relu`:return+(e>0)}}function Vd(e){let t=e.input*e.weight,n=t+e.bias,r=f(n,e.activation),i=r-e.target,a=i*i,o=2*i,s=Bd(n,r,e.activation),c=e.input,l=o*s*c,u=o*s*1,d=e.weight-e.learningRate*l,p=e.bias-e.learningRate*u,m=f(e.input*d+p,e.activation),h=m-e.target;return{...e,weightedInput:t,preActivation:n,prediction:r,error:i,loss:a,lossPredictionDerivative:o,activationDerivative:s,preActivationWeightDerivative:c,preActivationBiasDerivative:1,gradientWeight:l,gradientBias:u,nextWeight:d,nextBias:p,nextPrediction:m,nextLoss:h*h}}function X(e,t=5){return Number.isFinite(e)?Math.abs(e)<1e-12?`0`:Math.abs(e)>=1e3||Math.abs(e)>0&&Math.abs(e)<1e-4?e.toExponential(3):Number(e.toFixed(t)).toString():String(e)}var Hd=[{id:`example`,shortLabel:`Example`,title:`Choose one training example`,question:`What information is the neuron trying to connect?`,formula:e=>`x = ${X(e.input)}, target = ${X(e.target)}`,value:e=>`x ${X(e.input)} / target ${X(e.target)}`,explanation:()=>`The input is evidence. The target is the answer we want this one neuron to approach.`},{id:`multiply`,shortLabel:`Multiply`,title:`Scale the input by its weight`,question:`How strongly does this input contribute?`,formula:e=>`${X(e.input)} x ${X(e.weight)} = ${X(e.weightedInput)}`,value:e=>X(e.weightedInput),explanation:e=>`The current weight ${X(e.weight)} turns the input into one weighted contribution.`},{id:`bias`,shortLabel:`Add bias`,title:`Shift the weighted contribution`,question:`What should the neuron predict when its input contribution is zero?`,formula:e=>`${X(e.weightedInput)} + ${X(e.bias)} = ${X(e.preActivation)}`,value:e=>`z = ${X(e.preActivation)}`,explanation:e=>`The bias ${X(e.bias)} shifts the neuron before any activation is applied.`},{id:`activation`,shortLabel:`Activate`,title:`Transform the raw sum`,question:`What range or shape should the output have?`,formula:e=>`${e.activation}(${X(e.preActivation)}) = ${X(e.prediction)}`,value:e=>`prediction ${X(e.prediction)}`,explanation:e=>`The ${e.activation} activation transforms z into the value compared with the target.`},{id:`loss`,shortLabel:`Measure loss`,title:`Turn the mistake into one score`,question:`How wrong is the current prediction?`,formula:e=>`(${X(e.prediction)} - ${X(e.target)})^2 = ${X(e.loss)}`,value:e=>`loss ${X(e.loss)}`,explanation:e=>`The signed error is ${X(e.error)}. Squaring it makes the score positive and magnifies larger mistakes.`},{id:`backprop`,shortLabel:`Backprop`,title:`Assign responsibility with the chain rule`,question:`How much did each parameter contribute to the loss?`,formula:e=>`dL/dw = ${X(e.lossPredictionDerivative)} x ${X(e.activationDerivative)} x ${X(e.input)} = ${X(e.gradientWeight)}`,value:e=>`dw ${X(e.gradientWeight)} / db ${X(e.gradientBias)}`,explanation:()=>`Backpropagation multiplies local derivatives along each path from the loss to a parameter.`},{id:`update`,shortLabel:`Update`,title:`Move the parameters against the gradient`,question:`What small change should reduce the loss?`,formula:e=>`w' = ${X(e.weight)} - ${X(e.learningRate)} x ${X(e.gradientWeight)} = ${X(e.nextWeight)}`,value:e=>`w' ${X(e.nextWeight)} / b' ${X(e.nextBias)}`,explanation:e=>`With the proposed parameters, the loss changes from ${X(e.loss)} to ${X(e.nextLoss)}.`}];function Ud(e,t){let n=Number(e);return Number.isFinite(n)?n:t}function Wd(){let[e,t]=(0,l.useState)(zd),[n,r]=(0,l.useState)(0),[i,a]=(0,l.useState)(0),o=(0,l.useMemo)(()=>Vd(e),[e]),s=Hd[n];function c(e,n){t(t=>({...t,[e]:Ud(n,t[e])})),r(0)}function u(e){t(t=>({...t,activation:e})),r(0)}function d(){t(e=>{let t=Vd(e);return{...e,weight:Number(t.nextWeight.toPrecision(12)),bias:Number(t.nextBias.toPrecision(12))}}),a(e=>e+1),r(0)}function f(){t(zd),r(0),a(0)}return(0,E.jsxs)(`main`,{className:`workspace workspace--microscope`,children:[(0,E.jsxs)(`section`,{className:`microscope-stage`,"aria-label":`Training step microscope`,children:[(0,E.jsxs)(`div`,{className:`lab-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:`One neuron / one example / one update`}),(0,E.jsx)(`h2`,{children:`Training-step microscope`}),(0,E.jsx)(`p`,{children:`Reveal the arithmetic in order. Future phases stay hidden until you reach them.`})]}),(0,E.jsxs)(`div`,{className:`lab-chip`,children:[`update `,i]})]}),(0,E.jsx)(`ol`,{className:`phase-strip`,"aria-label":`Training phases`,children:Hd.map((e,t)=>(0,E.jsx)(`li`,{children:(0,E.jsxs)(`button`,{className:`phase-button${t===n?` phase-button--active`:``}${t<n?` phase-button--complete`:``}`,type:`button`,onClick:()=>r(t),"aria-current":t===n?`step`:void 0,children:[(0,E.jsx)(`span`,{children:t+1}),e.shortLabel]})},e.id))}),(0,E.jsxs)(`section`,{className:`microscope-focus`,"aria-live":`polite`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsxs)(`p`,{className:`eyebrow`,children:[`Phase `,n+1,` of `,Hd.length]}),(0,E.jsx)(`h2`,{children:s.title}),(0,E.jsx)(`p`,{className:`focus-question`,children:s.question})]}),(0,E.jsx)(`code`,{children:s.formula(o)}),(0,E.jsx)(`p`,{children:s.explanation(o)})]}),(0,E.jsx)(`section`,{className:`signal-pipeline`,"aria-label":`Neuron signal pipeline`,children:Hd.map((e,t)=>(0,E.jsxs)(`button`,{className:`signal-node${t===n?` signal-node--active`:``}${t>n?` signal-node--locked`:``}`,type:`button`,onClick:()=>r(t),children:[(0,E.jsx)(`span`,{children:e.shortLabel}),(0,E.jsx)(`strong`,{children:t<=n?e.value(o):`?`})]},e.id))}),s.id===`backprop`&&(0,E.jsxs)(`section`,{className:`derivative-panel`,"aria-label":`Chain rule factors`,children:[(0,E.jsxs)(`div`,{className:`derivative-factor`,children:[(0,E.jsx)(`span`,{children:`Loss slope`}),(0,E.jsxs)(`code`,{children:[`dL/dy = `,X(o.lossPredictionDerivative)]})]}),(0,E.jsx)(`div`,{className:`derivative-times`,"aria-hidden":`true`,children:`x`}),(0,E.jsxs)(`div`,{className:`derivative-factor`,children:[(0,E.jsx)(`span`,{children:`Activation slope`}),(0,E.jsxs)(`code`,{children:[`dy/dz = `,X(o.activationDerivative)]})]}),(0,E.jsx)(`div`,{className:`derivative-times`,"aria-hidden":`true`,children:`x`}),(0,E.jsxs)(`div`,{className:`derivative-factor`,children:[(0,E.jsx)(`span`,{children:`Weight path`}),(0,E.jsxs)(`code`,{children:[`dz/dw = `,X(o.preActivationWeightDerivative)]})]}),(0,E.jsx)(`div`,{className:`derivative-equals`,"aria-hidden":`true`,children:`=`}),(0,E.jsxs)(`div`,{className:`derivative-factor derivative-factor--result`,children:[(0,E.jsx)(`span`,{children:`Weight gradient`}),(0,E.jsxs)(`code`,{children:[`dL/dw = `,X(o.gradientWeight)]})]})]}),s.id===`update`&&(0,E.jsxs)(`section`,{className:`before-after`,"aria-label":`Parameter update result`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:`Before`}),(0,E.jsxs)(`strong`,{children:[`w `,X(o.weight),` / b `,X(o.bias)]}),(0,E.jsxs)(`small`,{children:[`prediction `,X(o.prediction),` / loss `,X(o.loss)]})]}),(0,E.jsx)(`div`,{className:`update-arrow`,"aria-hidden":`true`,children:`->`}),(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`span`,{children:`After proposed update`}),(0,E.jsxs)(`strong`,{children:[`w `,X(o.nextWeight),` / b `,X(o.nextBias)]}),(0,E.jsxs)(`small`,{children:[`prediction `,X(o.nextPrediction),` / loss `,X(o.nextLoss)]})]})]}),(0,E.jsxs)(`div`,{className:`microscope-actions`,children:[(0,E.jsx)(`button`,{type:`button`,disabled:n===0,onClick:()=>r(e=>Math.max(0,e-1)),children:`Previous phase`}),n<Hd.length-1?(0,E.jsx)(`button`,{className:`primary-action`,type:`button`,onClick:()=>r(e=>Math.min(Hd.length-1,e+1)),children:`Next phase`}):(0,E.jsx)(`button`,{className:`primary-action`,type:`button`,onClick:d,children:`Apply this update`}),(0,E.jsx)(`button`,{type:`button`,onClick:f,children:`Reset example`})]})]}),(0,E.jsxs)(`aside`,{className:`controls microscope-controls`,"aria-label":`Microscope values`,children:[(0,E.jsxs)(`div`,{className:`lesson`,children:[(0,E.jsx)(`span`,{children:`Change one thing`}),(0,E.jsx)(`p`,{children:`Adjust a value, then step forward again and watch where its effect first appears.`})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Input x`}),(0,E.jsx)(`input`,{"aria-label":`Input x`,type:`number`,step:`0.1`,value:e.input,onChange:e=>c(`input`,e.target.value)})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Target`}),(0,E.jsx)(`input`,{"aria-label":`Target`,type:`number`,step:`0.1`,value:e.target,onChange:e=>c(`target`,e.target.value)})]}),(0,E.jsxs)(`div`,{className:`field-grid`,children:[(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Weight w`}),(0,E.jsx)(`input`,{"aria-label":`Weight w`,type:`number`,step:`0.05`,value:e.weight,onChange:e=>c(`weight`,e.target.value)})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Bias b`}),(0,E.jsx)(`input`,{"aria-label":`Bias b`,type:`number`,step:`0.05`,value:e.bias,onChange:e=>c(`bias`,e.target.value)})]})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Activation`}),(0,E.jsxs)(`select`,{"aria-label":`Activation`,value:e.activation,onChange:e=>u(e.target.value),children:[(0,E.jsx)(`option`,{value:`linear`,children:`Identity / linear`}),(0,E.jsx)(`option`,{value:`sigmoid`,children:`Sigmoid`}),(0,E.jsx)(`option`,{value:`tanh`,children:`Tanh`}),(0,E.jsx)(`option`,{value:`relu`,children:`ReLU`})]})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Learning rate`}),(0,E.jsx)(`input`,{"aria-label":`Learning rate`,type:`number`,min:`0.0001`,step:`0.01`,value:e.learningRate,onChange:e=>c(`learningRate`,e.target.value)})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Current prediction`}),(0,E.jsx)(`strong`,{children:X(o.prediction)})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Current loss`}),(0,E.jsx)(`strong`,{children:X(o.loss)})]}),(0,E.jsxs)(`div`,{className:`gradients`,children:[(0,E.jsx)(`span`,{children:`Proposed gradients`}),(0,E.jsxs)(`code`,{children:[`dL/dw = `,X(o.gradientWeight)]}),(0,E.jsxs)(`code`,{children:[`dL/db = `,X(o.gradientBias)]})]})]})]})}var Gd={width:720,height:460,padLeft:64,padRight:24,padTop:26,padBottom:52},Kd=[`#237a57`,`#2563eb`,`#c2413b`,`#b7791f`,`#6d5bd0`];function qd(e,t=3){return Number.isFinite(e)?Math.abs(e)>=1e3?e.toFixed(0):Math.abs(e)<.01&&e!==0?e.toExponential(2):e.toFixed(t):`0`}function Jd(e,t,n){return Math.min(n,Math.max(t,e))}function Yd(e,t){let n=e.points.map(e=>e.x),r=[...e.points.map(e=>e.y),...tc(e.points,t),e.idealModel.weight*Math.min(...n)+e.idealModel.bias,e.idealModel.weight*Math.max(...n)+e.idealModel.bias],i=Math.min(...n),a=Math.max(...n),o=Math.min(...r),s=Math.max(...r),c=Math.max((a-i)*.12,1),l=Math.max((s-o)*.16,1);return{...Gd,xMin:i-c,xMax:a+c,yMin:o-l,yMax:s+l}}function Xd(e,t){let n=t.width-t.padLeft-t.padRight;return t.padLeft+(e-t.xMin)/(t.xMax-t.xMin)*n}function Zd(e,t){let n=t.height-t.padTop-t.padBottom;return t.padTop+(1-(e-t.yMin)/(t.yMax-t.yMin))*n}function Qd(e,t){let n=t.xMin,r=t.xMax,[i,a]=tc([{x:n,y:0},{x:r,y:0}],e);return`M ${Xd(n,t)} ${Zd(i??0,t)} L ${Xd(r,t)} ${Zd(a??0,t)}`}function $d(e){if(e.length===0)return``;let t=Math.max(...e.map(e=>e.loss),1),n=e[0].epoch,r=Math.max(e[e.length-1].epoch-n,1);return e.map((e,i)=>{let a=(e.epoch-n)/r*250,o=74-Jd(e.loss/t,0,1)*74;return`${i===0?`M`:`L`} ${a.toFixed(2)} ${o.toFixed(2)}`}).join(` `)}function ef(e){let t=Array.from({length:81},(e,t)=>-4+t*.1),n=t.map(t=>f(t,e)),r=Math.min(...n,-1),i=Math.max(...n,1);return t.map((e,t)=>{let a=(e+4)/8*250,o=82-(n[t]-r)/(i-r)*82;return`${t===0?`M`:`L`} ${a.toFixed(2)} ${o.toFixed(2)}`}).join(` `)}function tf(e,t,n){return{epoch:t.epoch,loss:nc(e.points,t,n),mae:rc(e.points,t),weight:t.weight,bias:t.bias}}function nf(e,t){return e===void 0?Kd[0]:Kd[Math.max(t.indexOf(e),0)%Kd.length]}function rf(){let[e,t]=(0,l.useState)(`microscope`),[n,r]=(0,l.useState)(Cc[0].id),i=Cc.find(e=>e.id===n)??Cc[0],[a,o]=(0,l.useState)(`linear`),[s,c]=(0,l.useState)(i.defaultLoss),[u,f]=(0,l.useState)(i.defaultLearningRate),[m,h]=(0,l.useState)(i.initialModel.weight),[g,_]=(0,l.useState)(i.initialModel.bias),[v,y]=(0,l.useState)(i.initialModel),[b,x]=(0,l.useState)([tf(i,i.initialModel,i.defaultLoss)]),[S,C]=(0,l.useState)(null),[w,ee]=(0,l.useState)(!1);(0,l.useEffect)(()=>{c(i.defaultLoss),f(i.defaultLearningRate),h(i.initialModel.weight),_(i.initialModel.bias),y(i.initialModel),C(null),ee(!1),x([tf(i,i.initialModel,i.defaultLoss)])},[i]);let te=(0,l.useMemo)(()=>tc(i.points,v),[v,i.points]),T=(0,l.useMemo)(()=>Yd(i,v),[v,i]),ne=(0,l.useMemo)(()=>nc(i.points,v,s),[s,v,i.points]),re=(0,l.useMemo)(()=>rc(i.points,v),[v,i.points]),ie=(0,l.useMemo)(()=>p(a),[a]),ae=(0,l.useMemo)(()=>Array.from(new Set(i.points.map(e=>e.group).filter(e=>e!==void 0))),[i.points]),oe=(0,l.useMemo)(()=>wc.map(e=>({category:e,labs:Cc.filter(t=>t.category===e)})),[]);function se(e){y(e.state),C(e),x(t=>[...t.slice(-159),{epoch:e.state.epoch,loss:e.loss,mae:e.mae,weight:e.state.weight,bias:e.state.bias}])}function ce(e){let t=oc(i.points,v,u,s,e),n=t[t.length-1];n!==void 0&&se(n)}function D(){let e={weight:m,bias:g,epoch:0};y(e),C(null),ee(!1),x([tf(i,e,s)])}return(0,l.useEffect)(()=>{if(!w)return;let e=window.setInterval(()=>{y(e=>{let t=ac(i.points,e,u,s);return C(t),x(e=>[...e.slice(-159),{epoch:t.state.epoch,loss:t.loss,mae:t.mae,weight:t.state.weight,bias:t.state.bias}]),t.state})},180);return()=>window.clearInterval(e)},[w,u,s,i.points]),(0,E.jsxs)(`div`,{className:`app`,children:[(0,E.jsxs)(`header`,{className:`app-header`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:e===`microscope`?`No hidden magic`:e===`optimization`?`Trust, then independently verify`:e===`convolution`?`One detector, every position`:e===`image-cnn`?`Channels become features`:e===`residual`?`Deep route, short route`:e===`recurrent`?`Memory becomes an input`:e===`attention`?`Every token asks and matches`:e===`representation`?`Compress, then reconstruct`:e===`structured`?`Structure shapes computation`:e===`deep`?`Scale shapes forward and backward signals`:e===`tensor`?`Forward reuses, backward sums`:e===`autograd`?`Record what ran, then reverse it`:e===`gradient-buffer`?`Backward adds, zero clears`:e===`forward-lowering`?`One graph, two executable IRs`:e===`training-lowering`?`Reverse and update become schedules`:e===`backend-parity`?`One graph can cross execution engines`:e===`precision-residency`?`Representation and movement are separate choices`:e===`linear`?`100-lab foundation`:`Hidden-layer playground`}),(0,E.jsx)(`h1`,{children:`ML Learning Lab`})]}),(0,E.jsxs)(`div`,{className:`header-actions`,children:[(0,E.jsxs)(`div`,{className:`mode-toggle`,"aria-label":`Workbench mode`,children:[(0,E.jsx)(`button`,{className:e===`microscope`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`microscope`),children:`Microscope`}),(0,E.jsx)(`button`,{className:e===`optimization`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`optimization`),children:`Optimization`}),(0,E.jsx)(`button`,{className:e===`linear`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`linear`),children:`Linear`}),(0,E.jsx)(`button`,{className:e===`hidden`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`hidden`),children:`Hidden Layer`}),(0,E.jsx)(`button`,{className:e===`convolution`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`convolution`),children:`Spatial`}),(0,E.jsx)(`button`,{className:e===`image-cnn`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`image-cnn`),children:`Image CNN`}),(0,E.jsx)(`button`,{className:e===`residual`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`residual`),children:`Residual`}),(0,E.jsx)(`button`,{className:e===`recurrent`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`recurrent`),children:`Recurrent`}),(0,E.jsx)(`button`,{className:e===`attention`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`attention`),children:`Attention`}),(0,E.jsx)(`button`,{className:e===`representation`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`representation`),children:`Representation`}),(0,E.jsx)(`button`,{className:e===`structured`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`structured`),children:`Structured`}),(0,E.jsx)(`button`,{className:e===`deep`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`deep`),children:`Deep Training`}),(0,E.jsx)(`button`,{className:e===`tensor`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`tensor`),children:`Tensor + Autograd`}),(0,E.jsx)(`button`,{className:e===`autograd`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`autograd`),children:`Autograd Graph`}),(0,E.jsx)(`button`,{className:e===`gradient-buffer`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`gradient-buffer`),children:`Grad Buffers`}),(0,E.jsx)(`button`,{className:e===`forward-lowering`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`forward-lowering`),children:`IR Lowering`}),(0,E.jsx)(`button`,{className:e===`training-lowering`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`training-lowering`),children:`Train Lowering`}),(0,E.jsx)(`button`,{className:e===`backend-parity`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`backend-parity`),children:`Backend Parity`}),(0,E.jsx)(`button`,{className:e===`precision-residency`?`mode-button mode-button--active`:`mode-button`,type:`button`,onClick:()=>t(`precision-residency`),children:`Precision + Residency`})]}),(0,E.jsx)(`div`,{className:`formula`,children:e===`microscope`?(0,E.jsxs)(E.Fragment,{children:[`forward `,`->`,` `,(0,E.jsx)(`strong`,{children:`loss`}),` `,`->`,` gradients `,`->`,` update`]}):e===`optimization`?(0,E.jsxs)(E.Fragment,{children:[`loss surface `,`->`,` `,(0,E.jsx)(`strong`,{children:`gradient check`}),` `,`->`,` batch strategy`]}):e===`convolution`?(0,E.jsxs)(E.Fragment,{children:[`window × `,(0,E.jsx)(`strong`,{children:`shared kernel`}),` `,`->`,` feature`]}):e===`image-cnn`?(0,E.jsxs)(E.Fragment,{children:[`channels `,`->`,` `,(0,E.jsx)(`strong`,{children:`normalize + ReLU`}),` `,`->`,` pool`]}):e===`residual`?(0,E.jsxs)(E.Fragment,{children:[`local layers + `,(0,E.jsx)(`strong`,{children:`identity skip`}),` `,`->`,` wider field`]}):e===`recurrent`?(0,E.jsxs)(E.Fragment,{children:[`input + `,(0,E.jsx)(`strong`,{children:`previous state`}),` `,`->`,` next state`]}):e===`attention`?(0,E.jsxs)(E.Fragment,{children:[`2 heads `,`->`,` `,(0,E.jsx)(`strong`,{children:`join`}),` `,`->`,` add + norm`]}):e===`representation`?(0,E.jsxs)(E.Fragment,{children:[`encode `,`->`,` `,(0,E.jsx)(`strong`,{children:`constrained latent`}),` `,`->`,` reconstruct`]}):e===`structured`?(0,E.jsxs)(E.Fragment,{children:[`connections `,`->`,` `,(0,E.jsx)(`strong`,{children:`shared rule`}),` `,`->`,` updated state`]}):e===`deep`?(0,E.jsxs)(E.Fragment,{children:[`initialize `,`->`,` `,(0,E.jsx)(`strong`,{children:`gradient flow`}),` `,`->`,` stabilize`]}):e===`tensor`?(0,E.jsxs)(E.Fragment,{children:[`align shapes `,`->`,` `,(0,E.jsx)(`strong`,{children:`reuse coordinates`}),` `,`->`,` reduce gradients`]}):e===`autograd`?(0,E.jsxs)(E.Fragment,{children:[`record operations `,`->`,` `,(0,E.jsx)(`strong`,{children:`save values`}),` `,`->`,` reverse graph`]}):e===`gradient-buffer`?(0,E.jsxs)(E.Fragment,{children:[`backward adds `,`->`,` `,(0,E.jsx)(`strong`,{children:`step reads`}),` `,`->`,` zero clears`]}):e===`forward-lowering`?(0,E.jsxs)(E.Fragment,{children:[`graph meaning `,`->`,` `,(0,E.jsx)(`strong`,{children:`NeuralIR schedule`}),` `,`->`,` MatrixIR fusion`]}):e===`training-lowering`?(0,E.jsxs)(E.Fragment,{children:[`saved values `,`->`,` `,(0,E.jsx)(`strong`,{children:`backward IR`}),` `,`->`,` optimizer IR`]}):e===`backend-parity`?(0,E.jsxs)(E.Fragment,{children:[`same graph `,`->`,` `,(0,E.jsx)(`strong`,{children:`CPU · Rust · WebGPU`}),` `,`->`,` equal output`]}):e===`precision-residency`?(0,E.jsxs)(E.Fragment,{children:[`number grid + `,(0,E.jsx)(`strong`,{children:`buffer placement`}),` `,`->`,` accuracy + transfers`]}):e===`linear`?(0,E.jsxs)(E.Fragment,{children:[`y = `,(0,E.jsx)(`strong`,{children:qd(v.weight)}),`x + `,(0,E.jsx)(`strong`,{children:qd(v.bias)})]}):(0,E.jsxs)(E.Fragment,{children:[`inputs `,`->`,` `,(0,E.jsx)(`strong`,{children:`hidden`}),` `,`->`,` prediction`]})})]})]}),e===`microscope`?(0,E.jsx)(Wd,{}):e===`optimization`?(0,E.jsx)(Uc,{}):e===`convolution`?(0,E.jsx)(gr,{}):e===`image-cnn`?(0,E.jsx)(Js,{}):e===`residual`?(0,E.jsx)(Wu,{}):e===`recurrent`?(0,E.jsx)(Hl,{}):e===`attention`?(0,E.jsx)(Ve,{}):e===`representation`?(0,E.jsx)(Iu,{}):e===`structured`?(0,E.jsx)(_d,{}):e===`deep`?(0,E.jsx)(Jr,{}):e===`tensor`?(0,E.jsx)(Rd,{}):e===`autograd`?(0,E.jsx)(gi,{}):e===`gradient-buffer`?(0,E.jsx)(aa,{}):e===`forward-lowering`?(0,E.jsx)(Hi,{}):e===`training-lowering`?(0,E.jsx)(jn,{}):e===`backend-parity`?(0,E.jsx)(tr,{}):e===`precision-residency`?(0,E.jsx)(_l,{}):e===`hidden`?(0,E.jsx)(Fs,{}):(0,E.jsxs)(`main`,{className:`workspace workspace--lab`,children:[(0,E.jsxs)(`nav`,{className:`lab-rail`,"aria-label":`ML lab examples`,children:[(0,E.jsxs)(`div`,{className:`rail-summary`,children:[(0,E.jsx)(`strong`,{children:Cc.length}),(0,E.jsx)(`span`,{children:`examples`})]}),oe.map(({category:e,labs:t})=>(0,E.jsxs)(`section`,{className:`lab-group`,children:[(0,E.jsx)(`h2`,{children:e}),(0,E.jsx)(`div`,{className:`lab-list`,children:t.map(e=>(0,E.jsxs)(`button`,{className:e.id===i.id?`lab-button lab-button--active`:`lab-button`,type:`button`,onClick:()=>r(e.id),children:[(0,E.jsx)(`span`,{children:e.title}),(0,E.jsx)(`small`,{children:e.source.kind===`local-csv`?`CSV`:`Synthetic`})]},e.id))})]},e))]}),(0,E.jsxs)(`section`,{className:`lab-stage`,"aria-label":`Selected lab`,children:[(0,E.jsxs)(`div`,{className:`lab-intro`,children:[(0,E.jsxs)(`div`,{children:[(0,E.jsx)(`p`,{className:`eyebrow`,children:i.category}),(0,E.jsx)(`h2`,{children:i.title}),(0,E.jsx)(`p`,{children:i.summary})]}),(0,E.jsxs)(`div`,{className:`lab-chip`,children:[i.points.length,` points`]})]}),(0,E.jsxs)(`section`,{className:`chart-panel`,"aria-label":`Training chart`,children:[(0,E.jsxs)(`svg`,{viewBox:`0 0 ${T.width} ${T.height}`,role:`img`,"aria-label":`${i.title} fit chart`,children:[(0,E.jsx)(`rect`,{className:`plot-bg`,x:T.padLeft,y:T.padTop,width:T.width-T.padLeft-T.padRight,height:T.height-T.padTop-T.padBottom}),[0,.25,.5,.75,1].map(e=>{let t=T.xMin+(T.xMax-T.xMin)*e,n=T.yMin+(T.yMax-T.yMin)*e;return(0,E.jsxs)(`g`,{children:[(0,E.jsx)(`line`,{className:`grid-line`,x1:Xd(t,T),x2:Xd(t,T),y1:T.padTop,y2:T.height-T.padBottom}),(0,E.jsx)(`text`,{className:`axis-label`,x:Xd(t,T),y:T.height-20,children:qd(t,1)}),(0,E.jsx)(`line`,{className:`grid-line`,x1:T.padLeft,x2:T.width-T.padRight,y1:Zd(n,T),y2:Zd(n,T)}),(0,E.jsx)(`text`,{className:`axis-label axis-label--y`,x:T.padLeft-10,y:Zd(n,T)+4,children:qd(n,1)})]},e)}),(0,E.jsx)(`path`,{className:`ideal-line`,d:Qd(i.idealModel,T)}),(0,E.jsx)(`path`,{className:`model-line`,d:Qd(v,T)}),i.points.map((e,t)=>{let n=Xd(e.x,T),r=Zd(e.y,T),i=Zd(te[t],T),a=nf(e.group,ae);return(0,E.jsxs)(`g`,{children:[(0,E.jsx)(`line`,{className:`error-line`,x1:n,x2:n,y1:r,y2:i}),(0,E.jsx)(`circle`,{className:`truth-point`,cx:n,cy:r,r:`6`,style:{fill:a}}),(0,E.jsx)(`circle`,{className:`prediction-point`,cx:n,cy:i,r:`5`})]},`${e.x}-${e.y}-${t}`)}),(0,E.jsx)(`text`,{className:`axis-title`,x:T.width/2,y:T.height-5,children:i.xLabel}),(0,E.jsx)(`text`,{className:`axis-title axis-title--y`,x:`20`,y:T.height/2,children:i.yLabel})]}),(0,E.jsxs)(`div`,{className:`legend`,"aria-label":`Chart legend`,children:[(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-dot legend-dot--truth`}),`Actual`]}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-dot legend-dot--prediction`}),`Prediction`]}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-line legend-line--model`}),`Current line`]}),(0,E.jsxs)(`span`,{children:[(0,E.jsx)(`i`,{className:`legend-line legend-line--ideal`}),`Best fit`]})]})]}),(0,E.jsx)(Xo,{model:v,lastStep:S,learningRate:u,lossKind:s,samplePoint:i.points[0],pointCount:i.points.length})]}),(0,E.jsxs)(`aside`,{className:`controls metrics`,"aria-label":`Training controls and metrics`,children:[(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Loss`}),(0,E.jsxs)(`select`,{value:s,onChange:e=>c(e.target.value),children:[(0,E.jsx)(`option`,{value:`mse`,children:`Mean squared error`}),(0,E.jsx)(`option`,{value:`mae`,children:`Mean absolute error`})]})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Activation preview`}),(0,E.jsx)(`select`,{value:a,onChange:e=>o(e.target.value),children:d.map(e=>(0,E.jsx)(`option`,{value:e.kind,children:e.label},e.kind))})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Learning rate`}),(0,E.jsx)(`input`,{type:`range`,min:i.learningRateMin,max:i.learningRateMax,step:i.learningRateStep,value:u,onChange:e=>f(Number(e.target.value))}),(0,E.jsx)(`input`,{type:`number`,min:i.learningRateMin,max:i.learningRateMax,step:i.learningRateStep,value:u,onChange:e=>f(Number(e.target.value))})]}),(0,E.jsxs)(`div`,{className:`field-grid`,children:[(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Initial weight`}),(0,E.jsx)(`input`,{type:`number`,step:`0.1`,value:m,onChange:e=>h(Number(e.target.value))})]}),(0,E.jsxs)(`label`,{className:`field`,children:[(0,E.jsx)(`span`,{children:`Initial bias`}),(0,E.jsx)(`input`,{type:`number`,step:`0.5`,value:g,onChange:e=>_(Number(e.target.value))})]})]}),(0,E.jsxs)(`div`,{className:`button-grid`,children:[(0,E.jsx)(`button`,{type:`button`,onClick:()=>ce(1),children:`Step`}),(0,E.jsx)(`button`,{type:`button`,onClick:()=>ce(25),children:`Step 25`}),(0,E.jsx)(`button`,{type:`button`,onClick:()=>ee(e=>!e),children:w?`Pause`:`Run`}),(0,E.jsx)(`button`,{type:`button`,onClick:D,children:`Reset`})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Epoch`}),(0,E.jsx)(`strong`,{children:v.epoch})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Loss`}),(0,E.jsx)(`strong`,{children:qd(ne,4)})]}),(0,E.jsxs)(`div`,{className:`metric`,children:[(0,E.jsx)(`span`,{children:`Average error`}),(0,E.jsx)(`strong`,{children:qd(re,3)})]}),(0,E.jsxs)(`div`,{className:`history`,children:[(0,E.jsxs)(`div`,{className:`history__topline`,children:[(0,E.jsx)(`span`,{children:`Loss history`}),(0,E.jsxs)(`strong`,{children:[b.length,` points`]})]}),(0,E.jsxs)(`svg`,{viewBox:`0 0 250 74`,role:`img`,"aria-label":`Loss history sparkline`,children:[(0,E.jsx)(`path`,{className:`history-grid`,d:`M 0 37 L 250 37`}),(0,E.jsx)(`path`,{className:`history-line`,d:$d(b)})]})]}),(0,E.jsxs)(`div`,{className:`gradients`,children:[(0,E.jsx)(`span`,{children:`Last gradient`}),(0,E.jsxs)(`code`,{children:[`w `,S===null?`0.000`:qd(S.gradientWeight,3)]}),(0,E.jsxs)(`code`,{children:[`b `,S===null?`0.000`:qd(S.gradientBias,3)]})]}),(0,E.jsxs)(`div`,{className:`lesson`,children:[(0,E.jsx)(`span`,{children:`Learning note`}),(0,E.jsx)(`p`,{children:i.lesson})]}),(0,E.jsxs)(`div`,{className:`activation-panel`,children:[(0,E.jsxs)(`div`,{className:`history__topline`,children:[(0,E.jsx)(`span`,{children:ie.label}),(0,E.jsx)(`strong`,{children:`f(x)`})]}),(0,E.jsxs)(`svg`,{viewBox:`0 0 250 82`,role:`img`,"aria-label":`${ie.label} activation curve`,children:[(0,E.jsx)(`path`,{className:`history-grid`,d:`M 0 41 L 250 41`}),(0,E.jsx)(`path`,{className:`activation-line`,d:ef(a)})]}),(0,E.jsx)(`p`,{children:ie.summary})]}),(0,E.jsxs)(`div`,{className:`source-panel`,children:[(0,E.jsx)(`span`,{children:`Data source`}),(0,E.jsx)(`p`,{children:i.source.name}),(0,E.jsx)(`code`,{children:i.source.license})]})]})]})]})}function af(e){return`ruleName`in e}var of=class extends Error{token;constructor(e,t){super(t?`Parse error at ${t.line}:${t.column}: ${e}`:`Parse error: ${e}`),this.name=`GrammarParseError`,this.token=t??null}},sf=class{tokens;grammar;pos;rules;ruleIndex;newlinesSignificant;memo;furthestPos;furthestExpected;_preParseHooks=[];_postParseHooks=[];trace;preserveSourceInfo;constructor(e,t,n){this.tokens=e,this.grammar=t,this.pos=0,this.memo=new Map,this.furthestPos=0,this.furthestExpected=[],this.trace=n?.trace??!1,this.preserveSourceInfo=n?.preserveSourceInfo??!1;let r=new Map,i=new Map;for(let e=0;e<t.rules.length;e++){let n=t.rules[e];r.set(n.name,n),i.set(n.name,e)}this.rules=r,this.ruleIndex=i,this.newlinesSignificant=this.grammarReferencesNewline()}isNewlinesSignificant(){return this.newlinesSignificant}addPreParse(e){this._preParseHooks.push(e)}addPostParse(e){this._postParseHooks.push(e)}parse(){if(this._preParseHooks.length>0){let e=[...this.tokens];for(let t of this._preParseHooks)e=t(e);this.tokens=e}if(this.grammar.rules.length===0)throw new of(`Grammar has no rules`);let e=this.grammar.rules[0],t=this.parseRule(e.name);if(t===null){let e=this.current();if(this.furthestExpected.length>0){let t=this.furthestExpected.join(` or `),n=this.furthestPos<this.tokens.length?this.tokens[this.furthestPos]:e;throw new of(`Expected ${t}, got ${JSON.stringify(n.value)}`,n)}throw new of(`Failed to parse`,e)}for(;this.pos<this.tokens.length&&this.current().type===`NEWLINE`;)this.pos++;if(this.pos<this.tokens.length&&this.current().type!==`EOF`){let e=this.current();if(this.furthestExpected.length>0&&this.furthestPos>this.pos){let t=this.furthestExpected.join(` or `),n=this.furthestPos<this.tokens.length?this.tokens[this.furthestPos]:e;throw new of(`Expected ${t}, got ${JSON.stringify(n.value)}`,n)}throw new of(`Unexpected token: ${JSON.stringify(e.value)}`,e)}let n=t;for(let e of this._postParseHooks)n=e(n);return n}current(){return this.pos<this.tokens.length?this.tokens[this.pos]:this.tokens[this.tokens.length-1]}recordFailure(e){this.pos>this.furthestPos?(this.furthestPos=this.pos,this.furthestExpected=[e]):this.pos===this.furthestPos&&(this.furthestExpected.includes(e)||this.furthestExpected.push(e))}grammarReferencesNewline(){for(let e of this.grammar.rules)if(this.elementReferencesNewline(e.body))return!0;return!1}elementReferencesNewline(e){switch(e.type){case`token_reference`:return e.name===`NEWLINE`;case`sequence`:return e.elements.some(e=>this.elementReferencesNewline(e));case`alternation`:return e.choices.some(e=>this.elementReferencesNewline(e));case`repetition`:case`optional`:case`group`:case`positive_lookahead`:case`negative_lookahead`:case`one_or_more`:return this.elementReferencesNewline(e.element);case`separated_repetition`:return this.elementReferencesNewline(e.element)||this.elementReferencesNewline(e.separator);default:return!1}}parseRule(e){let t=this.rules.get(e);if(!t)return null;let n=this.ruleIndex.get(e);if(n!==void 0){let t=`${n},${this.pos}`,r=this.memo.get(t);if(r!==void 0)return this.pos=r.endPos,r.ok?this.buildNode(e,r.children):null}let r=this.pos;if(n!==void 0){let e=`${n},${r}`;this.memo.set(e,{children:null,endPos:r,ok:!1})}if(this.trace){let t=this.current();process.stderr.write(`[TRACE] rule '${e}' at token ${r} (${t.type} "${t.value}") → `)}let i=this.matchElement(t.body);if(this.trace&&process.stderr.write(i===null?`fail
`:`match
`),n!==void 0){let e=`${n},${r}`;if(i===null?this.memo.set(e,{children:null,endPos:this.pos,ok:!1}):this.memo.set(e,{children:i,endPos:this.pos,ok:!0}),i!==null)for(;;){let n=this.pos;this.pos=r,this.memo.set(e,{children:i,endPos:n,ok:!0});let a=this.matchElement(t.body);if(a===null||this.pos<=n){this.pos=n,this.memo.set(e,{children:i,endPos:n,ok:!0});break}i=a}}return i===null?(this.pos=r,this.recordFailure(e),null):this.buildNode(e,i)}matchElement(e){let t=this.pos;switch(e.type){case`sequence`:{let n=[];for(let r of e.elements){let e=this.matchElement(r);if(e===null)return this.pos=t,null;n.push(...e)}return n}case`alternation`:for(let n of e.choices){this.pos=t;let e=this.matchElement(n);if(e!==null)return e}return this.pos=t,null;case`repetition`:{let t=[];for(;;){let n=this.pos,r=this.matchElement(e.element);if(r===null){this.pos=n;break}t.push(...r)}return t}case`optional`:{let t=this.matchElement(e.element);return t===null?[]:t}case`group`:return this.matchElement(e.element);case`token_reference`:return this.matchTokenReference(e.name);case`rule_reference`:{let n=this.parseRule(e.name);return n===null?(this.pos=t,null):[n]}case`literal`:{let t=this.current();if(!this.newlinesSignificant)for(;t.type===`NEWLINE`;)this.pos++,t=this.current();return t.value===e.value?(this.pos++,[t]):(this.recordFailure(`"${e.value}"`),null)}case`positive_lookahead`:{let n=this.matchElement(e.element);return this.pos=t,n===null?null:[]}case`negative_lookahead`:{let n=this.matchElement(e.element);return this.pos=t,n===null?[]:null}case`one_or_more`:{let n=this.matchElement(e.element);if(n===null)return this.pos=t,null;let r=[...n];for(;;){let t=this.pos,n=this.matchElement(e.element);if(n===null){this.pos=t;break}r.push(...n)}return r}case`separated_repetition`:{let n=this.matchElement(e.element);if(n===null)return this.pos=t,e.atLeastOne?null:[];let r=[...n];for(;;){let t=this.pos,n=this.matchElement(e.separator);if(n===null){this.pos=t;break}let i=this.matchElement(e.element);if(i===null){this.pos=t;break}r.push(...n,...i)}return r}default:return null}}matchTokenReference(e){let t=this.current();if(!this.newlinesSignificant&&e!==`NEWLINE`)for(;t.type===`NEWLINE`;)this.pos++,t=this.current();return t.type===e?(this.pos++,[t]):(this.recordFailure(e),null)}buildNode(e,t){let n=cf(t),r=this.preserveSourceInfo?lf(t):null;return{ruleName:e,children:t,...n??{},...r??{}}}};function cf(e){let t=uf(e),n=df(e);return!t||!n?null:{startLine:t.line,startColumn:t.column,endLine:n.line,endColumn:n.column}}function lf(e){let t=uf(e),n=df(e);if(!t||!n)return null;let r={};return t.startOffset!==void 0&&(r.startOffset=t.startOffset),n.endOffset!==void 0&&(r.endOffset=n.endOffset),t.tokenIndex!==void 0&&(r.firstTokenIndex=t.tokenIndex),n.tokenIndex!==void 0&&(r.lastTokenIndex=n.tokenIndex),t.leadingTrivia!==void 0&&(r.leadingTrivia=t.leadingTrivia),r}function uf(e){for(let t of e)if(af(t)){let e=uf(t.children);if(e)return e}else return t;return null}function df(e){for(let t=e.length-1;t>=0;t--){let n=e[t];if(af(n)){let e=df(n.children);if(e)return e}else return n}return null}var ff=class extends Error{line;column;constructor(e,t,n){super(`Lexer error at ${t}:${n}: ${e}`),this.name=`LexerError`,this.line=t,this.column=n}};function pf(e){return e.replace(/[.*+?^${}()|[\]\\]/g,`\\$&`)}function mf(e){return e.replace(/\(\?i:([^()]+)\)/g,(e,t)=>t.replace(/[A-Za-z]/g,e=>`[${e.toLowerCase()}${e.toUpperCase()}]`))}function hf(e,t){return new RegExp(mf(e),t)}function gf(e,t,n,r,i,a,o){if(e===`NAME`&&r.has(t))throw new ff(`Reserved keyword '${t}' cannot be used as an identifier`,a,o);return e===`NAME`&&n.has(t)?`KEYWORD`:i||e}function _f(e){let t=[],n=0;for(;n<e.length;)if(e[n]===`\\`&&n+1<e.length){let r={n:`
`,t:`	`,"\\":`\\`,'"':`"`},i=e[n+1];t.push(r[i]??i),n+=2}else t.push(e[n]),n+=1;return t.join(``)}var vf=class{_lexer;_source;_posAfter;_suppressed=!1;_emitted=[];_groupActions=[];_skipEnabled=null;_previousToken;_currentTokenLine;constructor(e,t,n,r,i){this._lexer=e,this._source=t,this._posAfter=n,this._previousToken=r,this._currentTokenLine=i}pushGroup(e){if(!this._lexer.hasGroup(e))throw Error(`Unknown pattern group: '${e}'. Available groups: ${this._lexer.availableGroups().sort().join(`, `)}`);this._groupActions.push([`push`,e])}popGroup(){this._groupActions.push([`pop`,``])}activeGroup(){return this._lexer.activeGroup()}groupStackDepth(){return this._lexer.groupStackDepth()}emit(e){this._emitted.push(e)}suppress(){this._suppressed=!0}peek(e=1){let t=this._posAfter+e-1;return t>=0&&t<this._source.length?this._source[t]:``}peekStr(e){return this._source.slice(this._posAfter,this._posAfter+e)}setSkipEnabled(e){this._skipEnabled=e}previousToken(){return this._previousToken}bracketDepth(e){return this._lexer.bracketDepth(e)}precededByNewline(){return this._previousToken===null?!1:this._previousToken.line<this._currentTokenLine}},yf=class{_source;_pos=0;_line=1;_column=1;_grammar;_keywordSet;_reservedSet;_hasSkipPatterns;_indentationMode;_layoutMode;_caseSensitive;_caseInsensitive;_patterns;_skipPatterns;_groupPatterns;_aliasMap;_groupStack=[`default`];_transitions;_startMode;_inheritingModes;_onToken=null;_skipEnabled=!0;_lastEmittedToken=null;_bracketDepths={paren:0,bracket:0,brace:0};_contextKeywordSet;_layoutKeywordSet;_preTokenizeHooks=[];_postTokenizeHooks=[];_preserveSourceInfo;_pendingTrivia=[];_nextTokenIndex=0;constructor(e,t,n){this._grammar=t,this._preserveSourceInfo=n?.preserveSourceInfo===!0,this._caseInsensitive=t.caseInsensitive===!0,this._caseSensitive=t.caseSensitive!==!1&&!this._caseInsensitive,this._source=!this._caseSensitive&&!this._caseInsensitive?e.toLowerCase():e,this._keywordSet=new Set(this._caseInsensitive?t.keywords.map(e=>e.toUpperCase()):t.keywords),this._reservedSet=new Set(t.reservedKeywords??[]),this._contextKeywordSet=new Set(t.contextKeywords??[]),this._indentationMode=t.mode===`indentation`,this._layoutMode=t.mode===`layout`,this._layoutKeywordSet=new Set(t.layoutKeywords??[]),this._hasSkipPatterns=(t.skipDefinitions??[]).length>0,this._aliasMap={};for(let e of t.definitions)e.alias&&(this._aliasMap[e.name]=e.alias);let r=this._caseInsensitive?`i`:``;if(this._patterns=t.definitions.map(e=>{let t=e.isRegex?e.pattern:pf(e.pattern);return{name:e.name,pattern:hf(t,r),alias:e.alias}}),this._skipPatterns=(t.skipDefinitions??[]).map(e=>{let t=e.isRegex?e.pattern:pf(e.pattern);return{name:e.name,pattern:hf(t,r)}}),this._groupPatterns={default:[...this._patterns]},t.groups)for(let[e,n]of Object.entries(t.groups)){let t=n.definitions.map(e=>{let t=e.isRegex?e.pattern:pf(e.pattern);return e.alias&&(this._aliasMap[e.name]=e.alias),{name:e.name,pattern:hf(t,r),alias:e.alias}});this._groupPatterns[e]=t}this._transitions=t.transitions??[];let i=t.startMode;this._startMode=i!==void 0&&(i==="default"||Object.prototype.hasOwnProperty.call(this._groupPatterns,i))?i:`default`;let a=new Set,o=new Set;for(let e of this._transitions)for(let t of e.actions)t.target!==void 0&&(t.kind===`push`&&a.add(t.target),t.kind===`set_mode`&&o.add(t.target));let s=new Set;for(let e of o)e!=="default"&&!a.has(e)&&s.add(e);this._inheritingModes=s,this._groupStack=[this._startMode]}setOnToken(e){this._onToken=e}hasGroup(e){return e in this._groupPatterns}availableGroups(){return Object.keys(this._groupPatterns)}activeGroup(){return this._groupStack[this._groupStack.length-1]}groupStackDepth(){return this._groupStack.length}bracketDepth(e){return e===void 0?this._bracketDepths.paren+this._bracketDepths.bracket+this._bracketDepths.brace:this._bracketDepths[e]}addPreTokenize(e){this._preTokenizeHooks.push(e)}addPostTokenize(e){this._postTokenizeHooks.push(e)}tokenize(){if(this._preTokenizeHooks.length>0){let e=this._source;for(let t of this._preTokenizeHooks)e=t(e);this._source=e}this._lastEmittedToken=null,this._bracketDepths={paren:0,bracket:0,brace:0},this._pendingTrivia=[],this._nextTokenIndex=0;let e;e=this._indentationMode?this._tokenizeIndentation():this._layoutMode?this._tokenizeLayout():this._tokenizeStandard();for(let t of this._postTokenizeHooks)e=t(e);return e}_tokenizeStandard(){let e=[];for(;this._pos<this._source.length;){let t=this._source[this._pos];if(this._hasSkipPatterns){if(this._skipEnabled&&this._trySkip())continue}else if(t===` `||t===`	`||t===`\r`){this._consumeDefaultWhitespace();continue}if(t===`
`){let t={type:`NEWLINE`,value:`\\n`,line:this._line,column:this._column},n=this._pos;this._advance(),this._emitToken(e,this._withOptionalSourceInfo(t,n));continue}let n=this._groupStack[this._groupStack.length-1],r=this._tryMatchTokenInGroup(n);if(r!==null){if(this._updateBracketDepth(r.value),this._onToken!==null){let t=new vf(this,this._source,this._pos,this._lastEmittedToken,r.line);this._onToken(r,t),t._suppressed||this._emitToken(e,r);for(let n of t._emitted)this._emitToken(e,n);for(let[e,n]of t._groupActions)e===`push`?this._groupStack.push(n):e===`pop`&&this._groupStack.length>1&&this._groupStack.pop();t._skipEnabled!==null&&(this._skipEnabled=t._skipEnabled),this._applyTransitions(r)}else this._emitToken(e,r),this._applyTransitions(r);continue}throw new ff(`Unexpected character: ${JSON.stringify(t)}`,this._line,this._column)}let t={type:`EOF`,value:``,line:this._line,column:this._column};return this._emitToken(e,this._withOptionalSourceInfo(t,this._pos)),this._groupStack=[this._startMode],this._skipEnabled=!0,e}_updateBracketDepth(e){if(e.length===1)switch(e){case`(`:this._bracketDepths.paren++;break;case`)`:this._bracketDepths.paren>0&&this._bracketDepths.paren--;break;case`[`:this._bracketDepths.bracket++;break;case`]`:this._bracketDepths.bracket>0&&this._bracketDepths.bracket--;break;case`{`:this._bracketDepths.brace++;break;case`}`:this._bracketDepths.brace>0&&this._bracketDepths.brace--;break}}_transitionKey(e){return e.type}_applyTransitions(e){if(this._transitions.length===0)return;let t=this._transitionKey(e),n=this._groupStack[this._groupStack.length-1]??`default`,r=null;for(let i of this._transitions)if(i.onTokens.includes(t)&&!(i.inMode!==void 0&&i.inMode!==n)&&!(i.onValue!==void 0&&i.onValue!==e.value)){r=i.actions;break}if(r!==null)for(let e of r)switch(e.kind){case`set_mode`:e.target!==void 0&&(this._groupStack[this._groupStack.length-1]=e.target);break;case`push`:e.target!==void 0&&this._groupStack.push(e.target);break;case`pop`:this._groupStack.length>1&&this._groupStack.pop();break;case`enable_skip`:this._skipEnabled=!0;break;case`disable_skip`:this._skipEnabled=!1;break}}_tokenizeIndentation(){let e=[],t=[0],n=0,r=!0;for(;this._pos<this._source.length;){if(r&&n===0){let n=this._processLineStart(t);if(n===`skip`)continue;for(let t of n)this._emitToken(e,t);if(r=!1,this._pos>=this._source.length)break}let i=this._source[this._pos];if(i===`
`){if(n===0){let t={type:`NEWLINE`,value:`\\n`,line:this._line,column:this._column},n=this._pos;this._advance(),this._emitToken(e,this._withOptionalSourceInfo(t,n))}else this._advance();r=!0;continue}if(n>0&&(i===` `||i===`	`||i===`\r`)){this._consumeDefaultWhitespace();continue}if(this._trySkip())continue;let a=this._tryMatchTokenInGroup(`default`);if(a!==null){a.value===`(`||a.value===`[`||a.value===`{`?n++:(a.value===`)`||a.value===`]`||a.value===`}`)&&n--,this._updateBracketDepth(a.value),this._emitToken(e,a),this._applyTransitions(a);continue}throw new ff(`Unexpected character: ${JSON.stringify(i)}`,this._line,this._column)}for(;t.length>1;)t.pop(),this._emitToken(e,this._withOptionalSourceInfo({type:`DEDENT`,value:``,line:this._line,column:this._column},this._pos));return(e.length===0||e[e.length-1].type!==`NEWLINE`)&&this._emitToken(e,this._withOptionalSourceInfo({type:`NEWLINE`,value:`\\n`,line:this._line,column:this._column},this._pos)),this._emitToken(e,this._withOptionalSourceInfo({type:`EOF`,value:``,line:this._line,column:this._column},this._pos)),this._groupStack=[this._startMode],this._skipEnabled=!0,e}_tokenizeLayout(){return this._applyLayout(this._tokenizeStandard())}_applyLayout(e){let t=[],n=[],r=0,i=0;for(let a=0;a<e.length;a++){let o=e[a],s=o.typeName??o.type;if(s===`NEWLINE`){t.push(o);let r=this._nextLayoutToken(e,a+1);if(i===0&&r!==null){for(;n.length>0&&r.column<n[n.length-1];)t.push(this._virtualLayoutToken(`VIRTUAL_RBRACE`,`}`,r)),n.pop();n.length>0&&(r.typeName??r.type)!==`EOF`&&r.value!==`}`&&r.column===n[n.length-1]&&t.push(this._virtualLayoutToken(`VIRTUAL_SEMICOLON`,`;`,r))}continue}if(s===`EOF`){for(;n.length>0;)t.push(this._virtualLayoutToken(`VIRTUAL_RBRACE`,`}`,o)),n.pop();t.push(o);continue}if(r>0)if(o.value===`{`)--r;else{for(let e=0;e<r;e++)n.push(o.column),t.push(this._virtualLayoutToken(`VIRTUAL_LBRACE`,`{`,o));r=0}t.push(o),this._isVirtualLayoutToken(o)||(o.value===`(`||o.value===`[`||o.value===`{`?i+=1:(o.value===`)`||o.value===`]`||o.value===`}`)&&i>0&&--i),this._isLayoutKeyword(o)&&(r+=1)}return t}_nextLayoutToken(e,t){for(let n=t;n<e.length;n++){let t=e[n];if((t.typeName??t.type)!==`NEWLINE`)return t}return null}_virtualLayoutToken(e,t,n){return this._withOptionalSourceInfo({type:e,typeName:e,value:t,line:n.line,column:n.column},n.startOffset??this._pos)}_isVirtualLayoutToken(e){return(e.typeName??e.type).startsWith(`VIRTUAL_`)}_isLayoutKeyword(e){if(this._layoutKeywordSet.size===0)return!1;let t=e.value??``;return this._layoutKeywordSet.has(t)||this._layoutKeywordSet.has(t.toLowerCase())}_processLineStart(e){let t=0,n=this._line,r=this._column,i=this._pos;for(;this._pos<this._source.length;){let e=this._source[this._pos];if(e===` `)t++,this._advance();else if(e===`	`)throw new ff(`Tab character in indentation (use spaces only)`,this._line,this._column);else break}if(t>0&&this._preserveSourceInfo&&this._pushTrivia(`WHITESPACE`,this._source.slice(i,this._pos),n,r,i),this._pos>=this._source.length)return`skip`;if(this._source[this._pos]===`
`){let e=this._line,t=this._column,n=this._pos;return this._advance(),this._pushTrivia(`NEWLINE`,`
`,e,t,n),`skip`}let a=this._source.slice(this._pos);for(let e of this._skipPatterns){let t=e.pattern.exec(a);if(t!==null&&t.index===0){let n=this._pos+t[0].length;if(n>=this._source.length||this._source[n]===`
`){let n=this._line,r=this._column,i=this._pos;for(let e=0;e<t[0].length;e++)this._advance();if(this._pushTrivia(e.name,t[0],n,r,i),this._pos<this._source.length&&this._source[this._pos]===`
`){let e=this._line,t=this._column,n=this._pos;this._advance(),this._pushTrivia(`NEWLINE`,`
`,e,t,n)}return`skip`}}}let o=e[e.length-1],s=[];if(t>o)e.push(t),s.push(this._withOptionalSourceInfo({type:`INDENT`,value:``,line:this._line,column:1},this._pos));else if(t<o){for(;e.length>1&&e[e.length-1]>t;)e.pop(),s.push(this._withOptionalSourceInfo({type:`DEDENT`,value:``,line:this._line,column:1},this._pos));if(e[e.length-1]!==t)throw new ff(`Inconsistent dedent`,this._line,this._column)}return s}_trySkip(){let e=this._source.slice(this._pos);for(let t of this._skipPatterns){let n=t.pattern.exec(e);if(n!==null&&n.index===0){let e=this._line,r=this._column,i=this._pos;for(let e=0;e<n[0].length;e++)this._advance();return this._pushTrivia(t.name,n[0],e,r,i),!0}}return!1}_tryMatchTokenInGroup(e){let t=this._source.slice(this._pos),n=Object.prototype.hasOwnProperty.call(this._groupPatterns,e)?this._groupPatterns[e]:this._patterns;if(e!=="default"&&this._inheritingModes.has(e)){let e=Object.prototype.hasOwnProperty.call(this._groupPatterns,`default`)?this._groupPatterns.default:this._patterns;n=n.concat(e)}for(let{name:e,pattern:r,alias:i}of n){let n=r.exec(t);if(n!==null&&n.index===0){let t=n[0],r=this._line,a=this._column,o=this._pos,s=this._caseInsensitive?t.toUpperCase():t,c=gf(e,s,this._keywordSet,this._reservedSet,i,r,a);if(this._caseInsensitive&&c===`KEYWORD`&&(t=s),(this._aliasMap[e]??e)===`STRING`||e===`STRING`||e.includes(`STRING`)||i&&i.includes(`STRING`)){if(t.length>=6&&(t.startsWith(`"""`)||t.startsWith(`'''`))){let e=t.slice(3,-3);t=this._grammar.escapeMode===`none`?e:_f(e)}else if(t.length>=2&&(t[0]===`"`||t[0]===`'`)){let e=t.slice(1,-1);t=this._grammar.escapeMode===`none`?e:_f(e)}}let l;c===`NAME`&&this._contextKeywordSet.size>0&&this._contextKeywordSet.has(t)&&(l=2);let u=l===void 0?{type:c,value:t,line:r,column:a}:{type:c,value:t,line:r,column:a,flags:l};for(let e=0;e<n[0].length;e++)this._advance();return this._withOptionalSourceInfo(u,o)}}return null}_consumeDefaultWhitespace(){let e=this._line,t=this._column,n=this._pos;for(;this._pos<this._source.length;){let e=this._source[this._pos];if(e!==` `&&e!==`	`&&e!==`\r`)break;this._advance()}this._pos>n&&this._pushTrivia(`WHITESPACE`,this._source.slice(n,this._pos),e,t,n)}_pushTrivia(e,t,n,r,i){this._preserveSourceInfo&&this._pendingTrivia.push({type:e,value:t,line:n,column:r,endLine:this._line,endColumn:this._column,startOffset:i,endOffset:this._pos})}_withOptionalSourceInfo(e,t){return this._preserveSourceInfo?{...e,startOffset:t,endOffset:this._pos,endLine:this._line,endColumn:this._column}:e}_emitToken(e,t){let n=t;this._preserveSourceInfo&&(n={...t,tokenIndex:this._nextTokenIndex++,...this._pendingTrivia.length>0?{leadingTrivia:[...this._pendingTrivia]}:{}},this._pendingTrivia=[]),e.push(n),this._lastEmittedToken=n}_advance(){this._pos<this._source.length&&(this._source[this._pos]===`
`?(this._line+=1,this._column=1):this._column+=1,this._pos+=1)}};function bf(e,t,n){return new yf(e,t,n).tokenize()}var xf={version:1,caseInsensitive:!1,caseSensitive:!0,definitions:[{name:`STRING_DQ`,pattern:`"([^"\\\\\\n]|\\\\.)*"`,isRegex:!0,lineNumber:66,alias:`STRING`},{name:`STRING_SQ`,pattern:`'([^'\\\\\\n]|\\\\.)*'`,isRegex:!0,lineNumber:67,alias:`STRING`},{name:`VARIABLE`,pattern:`\\$[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:83},{name:`PLACEHOLDER`,pattern:`%[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:93},{name:`DIMENSION`,pattern:`-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+`,isRegex:!0,lineNumber:102},{name:`PERCENTAGE`,pattern:`-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?%`,isRegex:!0,lineNumber:103},{name:`NUMBER`,pattern:`-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?`,isRegex:!0,lineNumber:104},{name:`HASH`,pattern:`#[a-zA-Z0-9_-]+`,isRegex:!0,lineNumber:110},{name:`AT_KEYWORD`,pattern:`@-?[a-zA-Z][a-zA-Z0-9-]*`,isRegex:!0,lineNumber:127},{name:`URL_TOKEN`,pattern:`url\\([^)'"]*\\)`,isRegex:!0,lineNumber:133},{name:`FUNCTION`,pattern:`-?[a-zA-Z_][a-zA-Z0-9_-]*\\(`,isRegex:!0,lineNumber:139},{name:`CDO`,pattern:`<!--`,isRegex:!1,lineNumber:145},{name:`CDC`,pattern:`-->`,isRegex:!1,lineNumber:146},{name:`UNICODE_RANGE`,pattern:`[Uu]\\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?`,isRegex:!0,lineNumber:152},{name:`CUSTOM_PROPERTY`,pattern:`--[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:153},{name:`IDENT`,pattern:`-?[a-zA-Z_][a-zA-Z0-9_-]*`,isRegex:!0,lineNumber:154},{name:`COLON_COLON`,pattern:`::`,isRegex:!1,lineNumber:163},{name:`TILDE_EQUALS`,pattern:`~=`,isRegex:!1,lineNumber:164},{name:`PIPE_EQUALS`,pattern:`|=`,isRegex:!1,lineNumber:165},{name:`CARET_EQUALS`,pattern:`^=`,isRegex:!1,lineNumber:166},{name:`DOLLAR_EQUALS`,pattern:`$=`,isRegex:!1,lineNumber:167},{name:`STAR_EQUALS`,pattern:`*=`,isRegex:!1,lineNumber:168},{name:`EQUALS_EQUALS`,pattern:`==`,isRegex:!1,lineNumber:171},{name:`NOT_EQUALS`,pattern:`!=`,isRegex:!1,lineNumber:172},{name:`GREATER_EQUALS`,pattern:`>=`,isRegex:!1,lineNumber:173},{name:`LESS_EQUALS`,pattern:`<=`,isRegex:!1,lineNumber:174},{name:`LBRACE`,pattern:`{`,isRegex:!1,lineNumber:180},{name:`RBRACE`,pattern:`}`,isRegex:!1,lineNumber:181},{name:`LPAREN`,pattern:`(`,isRegex:!1,lineNumber:182},{name:`RPAREN`,pattern:`)`,isRegex:!1,lineNumber:183},{name:`LBRACKET`,pattern:`[`,isRegex:!1,lineNumber:184},{name:`RBRACKET`,pattern:`]`,isRegex:!1,lineNumber:185},{name:`SEMICOLON`,pattern:`;`,isRegex:!1,lineNumber:186},{name:`COLON`,pattern:`:`,isRegex:!1,lineNumber:187},{name:`COMMA`,pattern:`,`,isRegex:!1,lineNumber:188},{name:`DOT`,pattern:`.`,isRegex:!1,lineNumber:189},{name:`PLUS`,pattern:`+`,isRegex:!1,lineNumber:190},{name:`GREATER`,pattern:`>`,isRegex:!1,lineNumber:191},{name:`LESS`,pattern:`<`,isRegex:!1,lineNumber:192},{name:`TILDE`,pattern:`~`,isRegex:!1,lineNumber:193},{name:`STAR`,pattern:`*`,isRegex:!1,lineNumber:194},{name:`PIPE`,pattern:`|`,isRegex:!1,lineNumber:195},{name:`BANG_DEFAULT`,pattern:`!default`,isRegex:!1,lineNumber:198},{name:`BANG_GLOBAL`,pattern:`!global`,isRegex:!1,lineNumber:199},{name:`BANG`,pattern:`!`,isRegex:!1,lineNumber:200},{name:`SLASH`,pattern:`/`,isRegex:!1,lineNumber:201},{name:`EQUALS`,pattern:`=`,isRegex:!1,lineNumber:202},{name:`AMPERSAND`,pattern:`&`,isRegex:!1,lineNumber:203},{name:`MINUS`,pattern:`-`,isRegex:!1,lineNumber:204}],keywords:[],mode:void 0,escapeMode:`none`,skipDefinitions:[{name:`LINE_COMMENT`,pattern:`\\/\\/[^\\n]*`,isRegex:!0,lineNumber:55},{name:`COMMENT`,pattern:`\\/\\*[\\s\\S]*?\\*\\/`,isRegex:!0,lineNumber:56},{name:`WHITESPACE`,pattern:`[ \\t\\r\\n]+`,isRegex:!0,lineNumber:57}],reservedKeywords:[],layoutKeywords:[],contextKeywords:[],errorDefinitions:[],groups:{}};function Sf(e){return bf(e,xf)}var Cf={version:1,rules:[{name:`stylesheet`,body:{type:`repetition`,element:{type:`rule_reference`,name:`rule`}},lineNumber:37},{name:`rule`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`lattice_rule`},{type:`rule_reference`,name:`at_rule`},{type:`rule_reference`,name:`qualified_rule`}]},lineNumber:39},{name:`lattice_rule`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`variable_declaration`},{type:`rule_reference`,name:`mixin_definition`},{type:`rule_reference`,name:`function_definition`},{type:`rule_reference`,name:`use_directive`},{type:`rule_reference`,name:`lattice_control`}]},lineNumber:51},{name:`variable_declaration`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`value_list`},{type:`optional`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`BANG_DEFAULT`},{type:`token_reference`,name:`BANG_GLOBAL`}]}},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:69},{name:`mixin_definition`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`literal`,value:`@mixin`},{type:`token_reference`,name:`FUNCTION`},{type:`optional`,element:{type:`rule_reference`,name:`mixin_params`}},{type:`token_reference`,name:`RPAREN`},{type:`rule_reference`,name:`block`}]},{type:`sequence`,elements:[{type:`literal`,value:`@mixin`},{type:`token_reference`,name:`IDENT`},{type:`rule_reference`,name:`block`}]}]},lineNumber:102},{name:`mixin_params`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`mixin_param`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`mixin_param`}]}}]},lineNumber:105},{name:`mixin_param`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`VARIABLE`},{type:`optional`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`mixin_value_list`}]}}]},lineNumber:112},{name:`mixin_value_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`mixin_value`},{type:`repetition`,element:{type:`rule_reference`,name:`mixin_value`}}]},lineNumber:117},{name:`mixin_value`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`UNICODE_RANGE`},{type:`rule_reference`,name:`function_call`},{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`}]},lineNumber:119},{name:`include_directive`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`literal`,value:`@include`},{type:`token_reference`,name:`FUNCTION`},{type:`optional`,element:{type:`rule_reference`,name:`include_args`}},{type:`token_reference`,name:`RPAREN`},{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`SEMICOLON`},{type:`rule_reference`,name:`block`}]}}]},{type:`sequence`,elements:[{type:`literal`,value:`@include`},{type:`token_reference`,name:`IDENT`},{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`SEMICOLON`},{type:`rule_reference`,name:`block`}]}}]}]},lineNumber:130},{name:`include_args`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`include_arg`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`include_arg`}]}}]},lineNumber:133},{name:`include_arg`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`value_list`}]},{type:`rule_reference`,name:`value_list`}]},lineNumber:137},{name:`lattice_control`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`if_directive`},{type:`rule_reference`,name:`for_directive`},{type:`rule_reference`,name:`each_directive`},{type:`rule_reference`,name:`while_directive`}]},lineNumber:160},{name:`if_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@if`},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`literal`,value:`@else`},{type:`literal`,value:`if`},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`}]}},{type:`optional`,element:{type:`sequence`,elements:[{type:`literal`,value:`@else`},{type:`rule_reference`,name:`block`}]}}]},lineNumber:164},{name:`for_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@for`},{type:`token_reference`,name:`VARIABLE`},{type:`literal`,value:`from`},{type:`rule_reference`,name:`lattice_expression`},{type:`group`,element:{type:`alternation`,choices:[{type:`literal`,value:`through`},{type:`literal`,value:`to`}]}},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`}]},lineNumber:171},{name:`each_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@each`},{type:`token_reference`,name:`VARIABLE`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`VARIABLE`}]}},{type:`literal`,value:`in`},{type:`rule_reference`,name:`each_list`},{type:`rule_reference`,name:`block`}]},lineNumber:176},{name:`each_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`value`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`value`}]}}]},lineNumber:179},{name:`while_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@while`},{type:`rule_reference`,name:`lattice_expression`},{type:`rule_reference`,name:`block`}]},lineNumber:184},{name:`lattice_expression`,body:{type:`rule_reference`,name:`lattice_or_expr`},lineNumber:203},{name:`lattice_or_expr`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_and_expr`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`literal`,value:`or`},{type:`rule_reference`,name:`lattice_and_expr`}]}}]},lineNumber:205},{name:`lattice_and_expr`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_comparison`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`literal`,value:`and`},{type:`rule_reference`,name:`lattice_comparison`}]}}]},lineNumber:207},{name:`lattice_comparison`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_additive`},{type:`optional`,element:{type:`sequence`,elements:[{type:`rule_reference`,name:`comparison_op`},{type:`rule_reference`,name:`lattice_additive`}]}}]},lineNumber:209},{name:`comparison_op`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`EQUALS_EQUALS`},{type:`token_reference`,name:`NOT_EQUALS`},{type:`token_reference`,name:`GREATER`},{type:`token_reference`,name:`GREATER_EQUALS`},{type:`token_reference`,name:`LESS`},{type:`token_reference`,name:`LESS_EQUALS`}]},lineNumber:211},{name:`lattice_additive`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_multiplicative`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`}]}},{type:`rule_reference`,name:`lattice_multiplicative`}]}}]},lineNumber:214},{name:`lattice_multiplicative`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`lattice_unary`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`SLASH`}]}},{type:`rule_reference`,name:`lattice_unary`}]}}]},lineNumber:219},{name:`lattice_unary`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`MINUS`},{type:`rule_reference`,name:`lattice_unary`}]},{type:`rule_reference`,name:`lattice_primary`}]},lineNumber:221},{name:`lattice_primary`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`literal`,value:`true`},{type:`literal`,value:`false`},{type:`literal`,value:`null`},{type:`rule_reference`,name:`function_call`},{type:`rule_reference`,name:`map_literal`},{type:`sequence`,elements:[{type:`token_reference`,name:`LPAREN`},{type:`rule_reference`,name:`lattice_expression`},{type:`token_reference`,name:`RPAREN`}]}]},lineNumber:224},{name:`map_literal`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LPAREN`},{type:`rule_reference`,name:`map_entry`},{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`map_entry`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`map_entry`}]}},{type:`token_reference`,name:`RPAREN`}]},lineNumber:235},{name:`map_entry`,body:{type:`sequence`,elements:[{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STRING`}]}},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`lattice_expression`}]},lineNumber:237},{name:`function_definition`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`literal`,value:`@function`},{type:`token_reference`,name:`FUNCTION`},{type:`optional`,element:{type:`rule_reference`,name:`mixin_params`}},{type:`token_reference`,name:`RPAREN`},{type:`rule_reference`,name:`function_body`}]},{type:`sequence`,elements:[{type:`literal`,value:`@function`},{type:`token_reference`,name:`IDENT`},{type:`rule_reference`,name:`function_body`}]}]},lineNumber:261},{name:`function_body`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACE`},{type:`repetition`,element:{type:`rule_reference`,name:`function_body_item`}},{type:`token_reference`,name:`RBRACE`}]},lineNumber:264},{name:`function_body_item`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`variable_declaration`},{type:`rule_reference`,name:`return_directive`},{type:`rule_reference`,name:`lattice_control`}]},lineNumber:266},{name:`return_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@return`},{type:`rule_reference`,name:`lattice_expression`},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:268},{name:`use_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@use`},{type:`token_reference`,name:`STRING`},{type:`optional`,element:{type:`sequence`,elements:[{type:`literal`,value:`as`},{type:`token_reference`,name:`IDENT`}]}},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:281},{name:`at_rule`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`AT_KEYWORD`},{type:`rule_reference`,name:`at_prelude`},{type:`group`,element:{type:`alternation`,choices:[{type:`token_reference`,name:`SEMICOLON`},{type:`rule_reference`,name:`block`}]}}]},lineNumber:294},{name:`at_prelude`,body:{type:`repetition`,element:{type:`rule_reference`,name:`at_prelude_token`}},lineNumber:296},{name:`at_prelude_token`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`UNICODE_RANGE`},{type:`token_reference`,name:`VARIABLE`},{type:`rule_reference`,name:`function_in_prelude`},{type:`rule_reference`,name:`paren_block`},{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`DOT`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`},{type:`token_reference`,name:`GREATER`},{type:`token_reference`,name:`TILDE`},{type:`token_reference`,name:`PIPE`},{type:`token_reference`,name:`EQUALS`},{type:`token_reference`,name:`AMPERSAND`},{type:`token_reference`,name:`CDO`},{type:`token_reference`,name:`CDC`}]},lineNumber:298},{name:`function_in_prelude`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`at_prelude_tokens`},{type:`token_reference`,name:`RPAREN`}]},lineNumber:306},{name:`paren_block`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LPAREN`},{type:`rule_reference`,name:`at_prelude_tokens`},{type:`token_reference`,name:`RPAREN`}]},lineNumber:307},{name:`at_prelude_tokens`,body:{type:`repetition`,element:{type:`rule_reference`,name:`at_prelude_token`}},lineNumber:308},{name:`qualified_rule`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`selector_list`},{type:`rule_reference`,name:`block`}]},lineNumber:314},{name:`selector_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`complex_selector`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`token_reference`,name:`COMMA`},{type:`rule_reference`,name:`complex_selector`}]}}]},lineNumber:320},{name:`complex_selector`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`compound_selector`},{type:`repetition`,element:{type:`sequence`,elements:[{type:`optional`,element:{type:`rule_reference`,name:`combinator`}},{type:`rule_reference`,name:`compound_selector`}]}}]},lineNumber:322},{name:`combinator`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`GREATER`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`TILDE`}]},lineNumber:324},{name:`compound_selector`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`rule_reference`,name:`simple_selector`},{type:`repetition`,element:{type:`rule_reference`,name:`subclass_selector`}}]},{type:`sequence`,elements:[{type:`rule_reference`,name:`subclass_selector`},{type:`repetition`,element:{type:`rule_reference`,name:`subclass_selector`}}]}]},lineNumber:326},{name:`simple_selector`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`AMPERSAND`},{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`PERCENTAGE`}]},lineNumber:331},{name:`subclass_selector`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`class_selector`},{type:`rule_reference`,name:`id_selector`},{type:`rule_reference`,name:`placeholder_selector`},{type:`rule_reference`,name:`attribute_selector`},{type:`rule_reference`,name:`pseudo_class`},{type:`rule_reference`,name:`pseudo_element`}]},lineNumber:334},{name:`placeholder_selector`,body:{type:`token_reference`,name:`PLACEHOLDER`},lineNumber:338},{name:`class_selector`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`DOT`},{type:`token_reference`,name:`IDENT`}]},lineNumber:340},{name:`id_selector`,body:{type:`token_reference`,name:`HASH`},lineNumber:342},{name:`attribute_selector`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACKET`},{type:`token_reference`,name:`IDENT`},{type:`optional`,element:{type:`sequence`,elements:[{type:`rule_reference`,name:`attr_matcher`},{type:`rule_reference`,name:`attr_value`},{type:`optional`,element:{type:`token_reference`,name:`IDENT`}}]}},{type:`token_reference`,name:`RBRACKET`}]},lineNumber:344},{name:`attr_matcher`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`EQUALS`},{type:`token_reference`,name:`TILDE_EQUALS`},{type:`token_reference`,name:`PIPE_EQUALS`},{type:`token_reference`,name:`CARET_EQUALS`},{type:`token_reference`,name:`DOLLAR_EQUALS`},{type:`token_reference`,name:`STAR_EQUALS`}]},lineNumber:346},{name:`attr_value`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`STRING`}]},lineNumber:349},{name:`pseudo_class`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`pseudo_class_args`},{type:`token_reference`,name:`RPAREN`}]},{type:`sequence`,elements:[{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`IDENT`}]}]},lineNumber:351},{name:`pseudo_class_args`,body:{type:`repetition`,element:{type:`rule_reference`,name:`pseudo_class_arg`}},lineNumber:354},{name:`pseudo_class_arg`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`DOT`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`COLON`},{type:`token_reference`,name:`AMPERSAND`},{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`pseudo_class_args`},{type:`token_reference`,name:`RPAREN`}]},{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACKET`},{type:`rule_reference`,name:`pseudo_class_args`},{type:`token_reference`,name:`RBRACKET`}]}]},lineNumber:356},{name:`pseudo_element`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`COLON_COLON`},{type:`token_reference`,name:`IDENT`}]},lineNumber:361},{name:`block`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`LBRACE`},{type:`rule_reference`,name:`block_contents`},{type:`token_reference`,name:`RBRACE`}]},lineNumber:371},{name:`block_contents`,body:{type:`repetition`,element:{type:`rule_reference`,name:`block_item`}},lineNumber:373},{name:`block_item`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`lattice_block_item`},{type:`rule_reference`,name:`at_rule`},{type:`rule_reference`,name:`declaration_or_nested`}]},lineNumber:375},{name:`lattice_block_item`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`variable_declaration`},{type:`rule_reference`,name:`include_directive`},{type:`rule_reference`,name:`lattice_control`},{type:`rule_reference`,name:`content_directive`},{type:`rule_reference`,name:`extend_directive`},{type:`rule_reference`,name:`at_root_directive`}]},lineNumber:381},{name:`content_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@content`},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:391},{name:`extend_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@extend`},{type:`rule_reference`,name:`selector_list`},{type:`token_reference`,name:`SEMICOLON`}]},lineNumber:399},{name:`at_root_directive`,body:{type:`sequence`,elements:[{type:`literal`,value:`@at-root`},{type:`group`,element:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`rule_reference`,name:`selector_list`},{type:`rule_reference`,name:`block`}]},{type:`rule_reference`,name:`block`}]}}]},lineNumber:404},{name:`declaration_or_nested`,body:{type:`alternation`,choices:[{type:`rule_reference`,name:`declaration`},{type:`rule_reference`,name:`qualified_rule`}]},lineNumber:406},{name:`declaration`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`rule_reference`,name:`property`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`value_list`},{type:`optional`,element:{type:`rule_reference`,name:`priority`}},{type:`token_reference`,name:`SEMICOLON`}]},{type:`sequence`,elements:[{type:`rule_reference`,name:`property`},{type:`token_reference`,name:`COLON`},{type:`rule_reference`,name:`block`}]}]},lineNumber:415},{name:`property`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`CUSTOM_PROPERTY`}]},lineNumber:418},{name:`priority`,body:{type:`sequence`,elements:[{type:`token_reference`,name:`BANG`},{type:`literal`,value:`important`}]},lineNumber:420},{name:`value_list`,body:{type:`sequence`,elements:[{type:`rule_reference`,name:`value`},{type:`repetition`,element:{type:`rule_reference`,name:`value`}}]},lineNumber:431},{name:`value`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`UNICODE_RANGE`},{type:`rule_reference`,name:`function_call`},{type:`token_reference`,name:`VARIABLE`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`},{type:`rule_reference`,name:`map_literal`}]},lineNumber:433},{name:`function_call`,body:{type:`alternation`,choices:[{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`function_args`},{type:`token_reference`,name:`RPAREN`}]},{type:`token_reference`,name:`URL_TOKEN`}]},lineNumber:439},{name:`function_args`,body:{type:`repetition`,element:{type:`rule_reference`,name:`function_arg`}},lineNumber:442},{name:`function_arg`,body:{type:`alternation`,choices:[{type:`token_reference`,name:`DIMENSION`},{type:`token_reference`,name:`PERCENTAGE`},{type:`token_reference`,name:`NUMBER`},{type:`token_reference`,name:`STRING`},{type:`token_reference`,name:`IDENT`},{type:`token_reference`,name:`HASH`},{type:`token_reference`,name:`CUSTOM_PROPERTY`},{type:`token_reference`,name:`COMMA`},{type:`token_reference`,name:`SLASH`},{type:`token_reference`,name:`PLUS`},{type:`token_reference`,name:`MINUS`},{type:`token_reference`,name:`STAR`},{type:`token_reference`,name:`VARIABLE`},{type:`sequence`,elements:[{type:`token_reference`,name:`FUNCTION`},{type:`rule_reference`,name:`function_args`},{type:`token_reference`,name:`RPAREN`}]}]},lineNumber:444}]};function wf(e){return new sf(Sf(e),Cf)}function Tf(e){return wf(e).parse()}var Ef=class extends Error{latticeMessage;line;column;constructor(e,t=0,n=0){let r=t?` at line ${t}, column ${n}`:``;super(`${e}${r}`),this.latticeMessage=e,this.line=t,this.column=n,Object.setPrototypeOf(this,new.target.prototype),this.name=new.target.name}};function Df(e,t){let n=e.length+1,r=t.length+1,i=Array.from({length:n},()=>Array(r).fill(0));for(let e=0;e<n;e+=1)i[e][0]=e;for(let e=0;e<r;e+=1)i[0][e]=e;for(let a=1;a<n;a+=1)for(let n=1;n<r;n+=1){let r=e[a-1]===t[n-1]?0:1;i[a][n]=Math.min(i[a-1][n]+1,i[a][n-1]+1,i[a-1][n-1]+r)}return i[n-1][r-1]}function Of(e,t){return[...t].map(t=>({candidate:t,distance:Df(e,t)})).filter(({candidate:t,distance:n})=>t.includes(e)||e.includes(t)||n<=3).sort((e,t)=>e.distance-t.distance||e.candidate.localeCompare(t.candidate)).slice(0,3).map(({candidate:e})=>e)}var kf=class extends Ef{name;constructor(e,t=0,n=0){super(`Undefined variable '${e}'`,t,n),this.name=e}},Af=class extends Ef{name;suggestions;constructor(e,t=0,n=0,r=[]){let i=Of(e,r),a=[`Undefined mixin '${e}'.`];r.length===0?a.push(`No mixins are currently defined in scope.`):i.length>0?a.push(`Did you mean ${i.map(e=>`'${e}'`).join(` or `)}?`):a.push(`Defined mixins in scope: ${r.sort().join(`, `)}.`),a.push("If this is a zero-argument mixin, both `@mixin card() { ... }` and `@mixin card { ... }` are valid."),super(a.join(` `),t,n),this.name=e,this.suggestions=i}},jf=class extends Ef{name;expected;got;constructor(e,t,n,r,i=0,a=0){super(`${e} '${t}' expects ${n} args, got ${r}`,i,a),this.name=t,this.expected=n,this.got=r}},Mf=class extends Ef{chain;constructor(e,t,n=0,r=0){let i=t.join(` → `);super(`Circular ${e}: ${i}`,n,r),this.chain=t}},Z=class extends Ef{op;leftType;rightType;constructor(e,t,n,r=0,i=0){super(`Cannot ${e} '${t}' and '${n}'`,r,i),this.op=e,this.leftType=t,this.rightType=n}},Nf=class extends Ef{name;constructor(e,t=0,n=0){super(`Function '${e}' has no @return`,t,n),this.name=e}},Pf=class extends Ef{maxIterations;constructor(e=1e3,t=0,n=0){super(`@while loop exceeded maximum iteration count (${e})`,t,n),this.maxIterations=e}},Ff=class extends Ef{constructor(e,t=0,n=0){super(e,t,n)}},If=class extends Ef{constructor(e=0,t=0){super(`Division by zero`,e,t)}},Lf=class e{bindings=new Map;parent;constructor(e=null){this.parent=e}get(e){if(this.bindings.has(e))return this.bindings.get(e);if(this.parent!==null)return this.parent.get(e)}set(e,t){this.bindings.set(e,t)}has(e){return this.bindings.has(e)?!0:this.parent===null?!1:this.parent.has(e)}hasLocal(e){return this.bindings.has(e)}setGlobal(e,t){let n=this;for(;n.parent!==null;)n=n.parent;n.set(e,t)}child(){return new e(this)}get depth(){return this.parent===null?0:1+this.parent.depth}toString(){let e=Array.from(this.bindings.keys());return`ScopeChain(depth=${this.depth}, bindings=[${e.join(`, `)}])`}};function Rf(e){return e>=`A`&&e<=`Z`||e>=`a`&&e<=`z`}function zf(e){let t=0;e[t]===`-`&&t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;let r=t-n,i=0;if(e[t]===`.`){t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;i=t-n}if(r===0&&i===0)return null;if(e[t]===`e`||e[t]===`E`){let n=t;t++,(e[t]===`+`||e[t]===`-`)&&t++;let r=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;t===r&&(t=n)}let a=e.slice(t);if(a.length===0)return null;for(let e of a)if(!Rf(e))return null;return{numberPart:e.slice(0,t),unit:a}}var Bf=class{value;kind=`number`;constructor(e){this.value=e}toString(){return this.value===Math.trunc(this.value)&&isFinite(this.value)?String(Math.trunc(this.value)):String(this.value)}},Vf=class{value;unit;kind=`dimension`;constructor(e,t){this.value=e,this.unit=t}toString(){return this.value===Math.trunc(this.value)&&isFinite(this.value)?`${Math.trunc(this.value)}${this.unit}`:`${this.value}${this.unit}`}},Hf=class{value;kind=`percentage`;constructor(e){this.value=e}toString(){return this.value===Math.trunc(this.value)&&isFinite(this.value)?`${Math.trunc(this.value)}%`:`${this.value}%`}},Uf=class{value;kind=`string`;constructor(e){this.value=e}toString(){return`"${this.value}"`}},Wf=class{value;kind=`ident`;constructor(e){this.value=e}toString(){return this.value}},Gf=class e{value;kind=`color`;constructor(e){this.value=e}toRgb(){let e=this.value.replace(/^#/,``);return e.length===3?[parseInt(e[0]+e[0],16),parseInt(e[1]+e[1],16),parseInt(e[2]+e[2],16),1]:e.length===6?[parseInt(e.slice(0,2),16),parseInt(e.slice(2,4),16),parseInt(e.slice(4,6),16),1]:e.length===8?[parseInt(e.slice(0,2),16),parseInt(e.slice(2,4),16),parseInt(e.slice(4,6),16),parseInt(e.slice(6,8),16)/255]:[0,0,0,1]}toHsl(){let[e,t,n,r]=this.toRgb(),i=e/255,a=t/255,o=n/255,s=Math.max(i,a,o),c=Math.min(i,a,o),l=(s+c)/2;if(s===c)return[0,0,l*100,r];let u=s-c,d=l>.5?u/(2-s-c):u/(s+c),f;return f=s===i?(a-o)/u+(a<o?6:0):s===a?(o-i)/u+2:(i-a)/u+4,f*=60,[f,d*100,l*100,r]}static fromRgb(t,n,r,i=1){return t=Math.max(0,Math.min(255,Math.round(t))),n=Math.max(0,Math.min(255,Math.round(n))),r=Math.max(0,Math.min(255,Math.round(r))),i=Math.max(0,Math.min(1,i)),i>=1?new e(`#${t.toString(16).padStart(2,`0`)}${n.toString(16).padStart(2,`0`)}${r.toString(16).padStart(2,`0`)}`):new e(`rgba(${t}, ${n}, ${r}, ${i})`)}static fromHsl(t,n,r,i=1){if(t=(t%360+360)%360,n=Math.max(0,Math.min(100,n))/100,r=Math.max(0,Math.min(100,r))/100,n===0){let t=Math.round(r*255);return e.fromRgb(t,t,t,i)}let a=r<.5?r*(1+n):r+n-r*n,o=2*r-a;function s(e,t,n){return n<0&&(n+=1),n>1&&--n,n<1/6?e+(t-e)*6*n:n<1/2?t:n<2/3?e+(t-e)*(2/3-n)*6:e}let c=t/360,l=Math.round(s(o,a,c+1/3)*255),u=Math.round(s(o,a,c)*255),d=Math.round(s(o,a,c-1/3)*255);return e.fromRgb(l,u,d,i)}toString(){return this.value}},Kf=class{value;kind=`bool`;constructor(e){this.value=e}toString(){return this.value?`true`:`false`}},qf=class{kind=`null`;toString(){return``}},Jf=class{items;kind=`list`;constructor(e){this.items=e}toString(){return this.items.map(e=>e.toString()).join(`, `)}},Yf=class{items;kind=`map`;constructor(e){this.items=e}get(e){for(let[t,n]of this.items)if(t===e)return n}keys(){return this.items.map(([e])=>e)}values(){return this.items.map(([,e])=>e)}hasKey(e){return this.items.some(([t])=>t===e)}toString(){return`(${this.items.map(([e,t])=>`${e}: ${t}`).join(`, `)})`}};function Xf(e){return e.kind===`bool`?e.value:!(e.kind===`null`||e.kind===`number`&&e.value===0)}function Zf(e){let{type:t,value:n}=e;if(t===`NUMBER`)return new Bf(parseFloat(n));if(t===`DIMENSION`){let e=zf(n);if(e)return new Vf(parseFloat(e.numberPart),e.unit);let t=0;for(n[t]===`-`&&t++;t<n.length&&(n[t]===`.`||n[t]>=`0`&&n[t]<=`9`);)t++;return new Vf(parseFloat(n.slice(0,t)),n.slice(t))}return t===`PERCENTAGE`?new Hf(parseFloat(n.replace(`%`,``))):t===`STRING`?new Uf(n):t===`HASH`?new Gf(n):t===`IDENT`?n===`true`?new Kf(!0):n===`false`?new Kf(!1):n===`null`?new qf:new Wf(n):new Wf(String(n))}function Qf(e){if(!(`ruleName`in e))return Zf(e);let t=e;for(let e of t.children)if(`ruleName`in e){let t=Qf(e);if(t.kind!==`null`)return t}else return Zf(e);return new qf}function $f(e){return e.toString()}function ep(e,t){if(e.kind===`number`&&t.kind===`number`)return new Bf(e.value+t.value);if(e.kind===`dimension`&&t.kind===`dimension`){if(e.unit===t.unit)return new Vf(e.value+t.value,e.unit);throw new Z(`add`,e.toString(),t.toString())}if(e.kind===`percentage`&&t.kind===`percentage`)return new Hf(e.value+t.value);if(e.kind===`string`&&t.kind===`string`)return new Uf(e.value+t.value);throw new Z(`add`,e.toString(),t.toString())}function tp(e,t){if(e.kind===`number`&&t.kind===`number`)return new Bf(e.value-t.value);if(e.kind===`dimension`&&t.kind===`dimension`){if(e.unit===t.unit)return new Vf(e.value-t.value,e.unit);throw new Z(`subtract`,e.toString(),t.toString())}if(e.kind===`percentage`&&t.kind===`percentage`)return new Hf(e.value-t.value);throw new Z(`subtract`,e.toString(),t.toString())}function np(e,t){if(e.kind===`number`&&t.kind===`number`)return new Bf(e.value*t.value);if(e.kind===`number`&&t.kind===`dimension`)return new Vf(e.value*t.value,t.unit);if(e.kind===`dimension`&&t.kind===`number`)return new Vf(e.value*t.value,e.unit);if(e.kind===`number`&&t.kind===`percentage`||e.kind===`percentage`&&t.kind===`number`)return new Hf(e.value*t.value);throw new Z(`multiply`,e.toString(),t.toString())}function rp(e,t){let n=()=>{if(t.kind===`number`||t.kind===`dimension`||t.kind===`percentage`){if(t.value===0)throw new If;return t.value}throw new Z(`divide`,e.toString(),t.toString())};if(e.kind===`number`&&t.kind===`number`){if(t.value===0)throw new If;return new Bf(e.value/t.value)}if(e.kind===`dimension`&&t.kind===`number`){if(t.value===0)throw new If;return new Vf(e.value/t.value,e.unit)}if(e.kind===`dimension`&&t.kind===`dimension`&&e.unit===t.unit){if(t.value===0)throw new If;return new Bf(e.value/t.value)}if(e.kind===`percentage`&&t.kind===`number`){if(t.value===0)throw new If;return new Hf(e.value/t.value)}throw n(),e.kind===`number`||e.kind===`dimension`||e.kind,new Z(`divide`,e.toString(),t.toString())}function ip(e){if(e.kind===`number`)return new Bf(-e.value);if(e.kind===`dimension`)return new Vf(-e.value,e.unit);if(e.kind===`percentage`)return new Hf(-e.value);throw new Z(`negate`,e.toString(),``)}function ap(e,t,n){if((e=>e.kind===`number`||e.kind===`dimension`||e.kind===`percentage`)(e)&&e.kind===t.kind){let r=e.value,i=t.value;if(e.kind===`dimension`&&t.kind===`dimension`&&e.unit!==t.unit&&n!==`EQUALS_EQUALS`&&n!==`NOT_EQUALS`)return new Kf(!1);switch(n){case`EQUALS_EQUALS`:return e.kind===`dimension`&&t.kind===`dimension`?new Kf(r===i&&e.unit===t.unit):new Kf(r===i);case`NOT_EQUALS`:return e.kind===`dimension`&&t.kind===`dimension`?new Kf(r!==i||e.unit!==t.unit):new Kf(r!==i);case`GREATER`:return new Kf(r>i);case`GREATER_EQUALS`:return new Kf(r>=i);case`LESS`:return new Kf(r<i);case`LESS_EQUALS`:return new Kf(r<=i)}}let r=e.toString(),i=t.toString();return n===`EQUALS_EQUALS`?new Kf(r===i):n===`NOT_EQUALS`?new Kf(r!==i):new Kf(!1)}function op(e){switch(e.kind){case`number`:case`dimension`:case`percentage`:return`number`;case`string`:case`ident`:return`string`;case`color`:return`color`;case`bool`:return`bool`;case`null`:return`null`;case`list`:return`list`;case`map`:return`map`;default:return`unknown`}}function sp(e){if(e.kind===`number`||e.kind===`dimension`||e.kind===`percentage`)return e.value;throw new Z(`use`,`Expected a number, got ${op(e)}`,``)}function cp(e){if(e.kind!==`color`)throw new Z(`use`,`Expected a color, got ${op(e)}`,``);return e}function lp(e){let t=sp(e);if(t<0||t>100)throw new Ff(`Amount must be between 0% and 100%`);return t}function up(e){if(e.kind!==`map`)throw new Z(`use`,`Expected a map, got ${op(e)}`,``);return e}var dp=e=>{if(e.length<2)throw new Z(`call`,`map-get requires 2 arguments`,``);let t=up(e[0]),n=e[1].toString().replace(/^"|"$/g,``);return t.get(n)??new qf},fp=e=>{if(!e.length)throw new Z(`call`,`map-keys requires 1 argument`,``);return new Jf(up(e[0]).keys().map(e=>new Wf(e)))},pp=e=>{if(!e.length)throw new Z(`call`,`map-values requires 1 argument`,``);return new Jf(up(e[0]).values())},mp=e=>{if(e.length<2)throw new Z(`call`,`map-has-key requires 2 arguments`,``);let t=up(e[0]),n=e[1].toString().replace(/^"|"$/g,``);return new Kf(t.hasKey(n))},hp=e=>{if(e.length<2)throw new Z(`call`,`map-merge requires 2 arguments`,``);let t=up(e[0]),n=up(e[1]),r=new Map;for(let[e,n]of t.items)r.set(e,n);for(let[e,t]of n.items)r.set(e,t);return new Yf(Array.from(r.entries()))},gp=e=>{if(!e.length)throw new Z(`call`,`map-remove requires at least 1 argument`,``);let t=up(e[0]),n=new Set(e.slice(1).map(e=>e.toString().replace(/^"|"$/g,``)));return new Yf(t.items.filter(([e])=>!n.has(e)))},_p=e=>{let t=cp(e[0]),n=lp(e[1]),[r,i,a,o]=t.toHsl();return Gf.fromHsl(r,i,Math.min(100,a+n),o)},vp=e=>{let t=cp(e[0]),n=lp(e[1]),[r,i,a,o]=t.toHsl();return Gf.fromHsl(r,i,Math.max(0,a-n),o)},yp=e=>{let t=cp(e[0]),n=lp(e[1]),[r,i,a,o]=t.toHsl();return Gf.fromHsl(r,Math.min(100,i+n),a,o)},bp=e=>{let t=cp(e[0]),n=lp(e[1]),[r,i,a,o]=t.toHsl();return Gf.fromHsl(r,Math.max(0,i-n),a,o)},xp=e=>{let t=cp(e[0]),n=sp(e[1]),[r,i,a,o]=t.toHsl();return Gf.fromHsl((r+n)%360,i,a,o)},Sp=e=>{let[t,n,r,i]=cp(e[0]).toHsl();return Gf.fromHsl((t+180)%360,n,r,i)},Cp=e=>{let t=cp(e[0]),n=cp(e[1]),r=(e.length>=3?sp(e[2]):50)/100,[i,a,o,s]=t.toRgb(),[c,l,u,d]=n.toRgb();return Gf.fromRgb(Math.round(i*r+c*(1-r)),Math.round(a*r+l*(1-r)),Math.round(o*r+u*(1-r)),s*r+d*(1-r))},wp=e=>{if(e.length===2&&e[0].kind===`color`){let t=e[0],n=sp(e[1]),[r,i,a]=t.toRgb();return Gf.fromRgb(r,i,a,n)}return e.length===4?Gf.fromRgb(Math.round(sp(e[0])),Math.round(sp(e[1])),Math.round(sp(e[2])),sp(e[3])):new qf},Tp=e=>{let[t]=cp(e[0]).toRgb();return new Bf(t)},Ep=e=>{let[,t]=cp(e[0]).toRgb();return new Bf(t)},Dp=e=>{let[,,t]=cp(e[0]).toRgb();return new Bf(t)},Op=e=>{let[t]=cp(e[0]).toHsl();return new Vf(Math.round(t),`deg`)},kp=e=>{let[,t]=cp(e[0]).toHsl();return new Hf(Math.round(t))},Ap=e=>{let[,,t]=cp(e[0]).toHsl();return new Hf(Math.round(t))},jp=e=>{if(e.length<2)throw new Z(`call`,`nth requires 2 arguments`,``);let t=e[0],n=Math.trunc(sp(e[1]));if(n<1)throw new Ff(`List index must be 1 or greater`);if(t.kind===`list`){if(n>t.items.length)throw new Ff(`Index ${n} out of bounds for list of length ${t.items.length}`);return t.items[n-1]}if(n===1)return t;throw new Ff(`Index ${n} out of bounds for list of length 1`)},Mp=e=>{if(!e.length)throw new Z(`call`,`length requires 1 argument`,``);let t=e[0];return t.kind===`list`||t.kind===`map`?new Bf(t.items.length):new Bf(1)},Np=e=>{if(e.length<2)throw new Z(`call`,`join requires at least 2 arguments`,``);let t=e[0].kind===`list`?e[0].items:[e[0]],n=e[1].kind===`list`?e[1].items:[e[1]];return new Jf([...t,...n])},Pp=e=>{if(e.length<2)throw new Z(`call`,`append requires at least 2 arguments`,``);let t=e[0].kind===`list`?[...e[0].items]:[e[0]];return t.push(e[1]),new Jf(t)},Fp=e=>{if(e.length<2)throw new Z(`call`,`index requires 2 arguments`,``);let t=e[0].kind===`list`?e[0].items:[e[0]],n=e[1].toString();for(let e=0;e<t.length;e++)if(t[e].toString()===n)return new Bf(e+1);return new qf},Ip=e=>{if(!e.length)throw new Z(`call`,`type-of requires 1 argument`,``);return new Uf(op(e[0]))},Lp=e=>{if(!e.length)throw new Z(`call`,`unit requires 1 argument`,``);let t=e[0];if(t.kind===`dimension`)return new Uf(t.unit);if(t.kind===`percentage`)return new Uf(`%`);if(t.kind===`number`)return new Uf(``);throw new Z(`use`,`Expected a number, got ${op(t)}`,``)},Rp=e=>{if(!e.length)throw new Z(`call`,`unitless requires 1 argument`,``);return new Kf(e[0].kind===`number`)},zp=e=>{if(e.length<2)throw new Z(`call`,`comparable requires 2 arguments`,``);let t=e[0],n=e[1];if(t.kind===n.kind)return t.kind===`dimension`&&n.kind===`dimension`?new Kf(t.unit===n.unit):new Kf(!0);let r=e=>e===`number`||e===`dimension`||e===`percentage`;return r(t.kind)&&r(n.kind)&&(t.kind===`number`||n.kind===`number`)?new Kf(!0):new Kf(!1)},Bp=e=>{if(e.length<2)throw new Z(`call`,`math.div requires 2 arguments`,``);let t=e[0],n=e[1],r=sp(n);if(r===0)throw new If;let i=sp(t);return t.kind===`dimension`&&n.kind===`number`?new Vf(i/r,t.unit):t.kind===`dimension`&&n.kind===`dimension`&&t.unit===n.unit?new Bf(i/r):t.kind===`percentage`&&n.kind===`number`?new Hf(i/r):new Bf(i/r)};function Vp(e){return t=>{if(!t.length)throw new Z(`call`,`math function requires 1 argument`,``);let n=t[0],r=e(sp(n));return n.kind===`dimension`?new Vf(r,n.unit):n.kind===`percentage`?new Hf(r):new Bf(r)}}var Hp=Vp(Math.floor),Up=Vp(Math.ceil),Wp=Vp(Math.round),Gp=Vp(Math.abs),Kp=new Map([[`map-get`,dp],[`map-keys`,fp],[`map-values`,pp],[`map-has-key`,mp],[`map-merge`,hp],[`map-remove`,gp],[`lighten`,_p],[`darken`,vp],[`saturate`,yp],[`desaturate`,bp],[`adjust-hue`,xp],[`complement`,Sp],[`mix`,Cp],[`rgba`,wp],[`red`,Tp],[`green`,Ep],[`blue`,Dp],[`hue`,Op],[`saturation`,kp],[`lightness`,Ap],[`nth`,jp],[`length`,Mp],[`join`,Np],[`append`,Pp],[`index`,Fp],[`type-of`,Ip],[`unit`,Lp],[`unitless`,Rp],[`comparable`,zp],[`math.div`,Bp],[`math.floor`,Hp],[`math.ceil`,Up],[`math.round`,Wp],[`math.abs`,Gp],[`math.min`,e=>{if(!e.length)throw new Z(`call`,`math.min requires at least 1 argument`,``);let t=e[0],n=sp(t);for(let r=1;r<e.length;r++){let i=sp(e[r]);i<n&&(t=e[r],n=i)}return t}],[`math.max`,e=>{if(!e.length)throw new Z(`call`,`math.max requires at least 1 argument`,``);let t=e[0],n=sp(t);for(let r=1;r<e.length;r++){let i=sp(e[r]);i>n&&(t=e[r],n=i)}return t}]]);function qp(e){return`ruleName`in e}function Jp(e){return e.type}var Yp=class{scope;constructor(e){this.scope=e}evaluate(e){if(!qp(e))return Zf(e);switch(e.ruleName){case`lattice_expression`:return this._evalLatticeExpression(e);case`lattice_or_expr`:return this._evalOrExpr(e);case`lattice_and_expr`:return this._evalAndExpr(e);case`lattice_comparison`:return this._evalComparison(e);case`lattice_additive`:return this._evalAdditive(e);case`lattice_multiplicative`:return this._evalMultiplicative(e);case`lattice_unary`:return this._evalUnary(e);case`lattice_primary`:return this._evalPrimary(e);case`comparison_op`:return Zf(e.children[0]);case`value_list`:return this._evalValueList(e)}let t=e.children;if(t.length===1)return this.evaluate(t[0]);for(let e of t)if(qp(e)||e.type)return this.evaluate(e);return new qf}_evalLatticeExpression(e){return this.evaluate(e.children[0])}_evalOrExpr(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!qp(e)&&e.value===`or`){r++;continue}if(Xf(n))return n;n=this.evaluate(e),r++}return n}_evalAndExpr(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!qp(e)&&e.value===`and`){r++;continue}if(!Xf(n))return n;n=this.evaluate(e),r++}return n}_evalComparison(e){let t=e.children,n=this.evaluate(t[0]);if(t.length===1)return n;let r=null,i=null;for(let e=1;e<t.length;e++){let n=t[e];if(qp(n)&&n.ruleName===`comparison_op`)r=n;else if(r!==null){i=n;break}}if(r===null||i===null)return n;let a=this.evaluate(i),o=r.children[0];return ap(n,a,Jp(o))}_evalValueList(e){let t=e.children;return t.length<=1?t.length===0?new qf:this.evaluate(t[0]):t.some(e=>!qp(e)&&e.value!==void 0&&[`+`,`-`,`*`].includes(e.value))?this._evalAdditive(e):this.evaluate(t[0])}_evalAdditive(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!qp(e)){let i=e.value;if((i===`+`||i===`-`)&&(r++,r<t.length)){let e=this.evaluate(t[r]);n=i===`+`?ep(n,e):tp(n,e)}}r++}return n}_evalMultiplicative(e){let t=e.children,n=this.evaluate(t[0]),r=1;for(;r<t.length;){let e=t[r];if(!qp(e)){let i=e.value;if((i===`*`||i===`/`)&&(r++,r<t.length)){let e=this.evaluate(t[r]);n=i===`*`?np(n,e):rp(n,e)}}r++}return n}_evalUnary(e){let t=e.children;return t.length>=2&&!qp(t[0])&&t[0].value===`-`?ip(this.evaluate(t[1])):this.evaluate(t[0])}_evalPrimary(e){let t=e.children;for(let e of t){if(!qp(e)){let t=e,n=Jp(t);if(n===`LPAREN`||n===`RPAREN`)continue;if(n===`VARIABLE`){let e=this.scope.get(t.value);return e===void 0?new Wf(t.value):typeof e==`object`&&e&&`kind`in e?e:typeof e==`object`&&e&&`ruleName`in e?Qf(e):typeof e==`object`&&e&&`type`in e?Zf(e):new qf}return Zf(t)}return this.evaluate(e)}return new qf}};function Q(e){return`ruleName`in e}function Xp(e){return e.type}function Zp(e){if(!Q(e))return e.value}var Qp=new Set(`rgb.rgba.hsl.hsla.hwb.lab.lch.oklch.oklab.color.color-mix.calc.min.max.clamp.abs.sign.round.mod.rem.sin.cos.tan.asin.acos.atan.atan2.pow.sqrt.hypot.log.exp.var.env.url.format.local.linear-gradient.radial-gradient.conic-gradient.repeating-linear-gradient.repeating-radial-gradient.repeating-conic-gradient.counter.counters.attr.element.translate.translateX.translateY.translateZ.rotate.rotateX.rotateY.rotateZ.scale.scaleX.scaleY.scaleZ.skew.skewX.skewY.matrix.matrix3d.perspective.cubic-bezier.steps.path.polygon.circle.ellipse.inset.image-set.cross-fade.fit-content.minmax.repeat.blur.brightness.contrast.drop-shadow.grayscale.hue-rotate.invert.opacity.saturate.sepia`.split(`.`));function $p(e){let t=e.replace(/\($/,``);return Qp.has(t)}var em=class{value;constructor(e){this.value=e}},tm=class{ruleName;children;constructor(e,t){this.ruleName=e,this.children=t}};function nm(e){return e>=`A`&&e<=`Z`||e>=`a`&&e<=`z`}function rm(e){let t=0;e[t]===`-`&&t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;let r=t-n,i=0;if(e[t]===`.`){t++;let n=t;for(;t<e.length&&e[t]>=`0`&&e[t]<=`9`;)t++;i=t-n}return r===0&&i===0?-1:t}function im(e){if(e.length===0)return!1;for(let t of e)if(!nm(t))return!1;return!0}function am(e,t){let n=Q(t)?0:t.line??0,r=Q(t)?0:t.column??0,i=`IDENT`;if(e.startsWith(`#`))i=`HASH`;else if(e.startsWith(`"`)||e.startsWith(`'`))i=`STRING`;else{let t=rm(e);t===e.length-1&&e[t]===`%`?i=`PERCENTAGE`:t>0&&t<e.length&&im(e.slice(t))?i=`DIMENSION`:t===e.length&&(i=`NUMBER`)}return{type:i,value:e,line:n,column:r}}function om(e,t){return new tm(`value`,[am(e,t)])}function sm(e){if(!Q(e))return{...e};let t=e;return new tm(t.ruleName,t.children.map(e=>sm(e)))}function $(e){return e.children}function cm(e,t){e.children=t}var lm=class{variables=new Lf;mixins=new Map;functions=new Map;mixinStack=[];functionStack=[];maxWhileIterations;extendMap=new Map;atRootRules=[];contentBlockStack=[];contentScopeStack=[];constructor(e=1e3){this.maxWhileIterations=e}transform(e){this._collectSymbols(e);let t=this._expandNode(e,this.variables),n=this._cleanup(t);return this.extendMap.size>0&&this._applyExtends(n),this.atRootRules.length>0&&this._spliceAtRootRules(n),n}_collectSymbols(e){if(!Q(e))return;let t=[];for(let n of $(e)){if(!Q(n)){t.push(n);continue}let e=n;if(e.ruleName===`rule`){let r=$(e);if(r.length===0){t.push(n);continue}let i=r[0];if(!Q(i)){t.push(n);continue}let a=i;if(a.ruleName===`lattice_rule`){let e=$(a);if(e.length===0){t.push(n);continue}let r=e[0];if(!Q(r)){t.push(n);continue}let i=r.ruleName;if(i===`variable_declaration`){this._collectVariable(r);continue}else if(i===`mixin_definition`){this._collectMixin(r);continue}else if(i===`function_definition`){this._collectFunction(r);continue}else if(i===`use_directive`)continue}t.push(n)}else t.push(n)}cm(e,t)}_collectVariable(e){let t,n,r=!1,i=!1;for(let a of $(e))if(Q(a)){let e=a;if(e.ruleName===`value_list`)n=e;else if(e.ruleName===`variable_flag`){for(let t of $(e))if(!Q(t)){let e=Xp(t);e===`BANG_DEFAULT`?r=!0:e===`BANG_GLOBAL`&&(i=!0)}}}else{let e=Xp(a);e===`VARIABLE`?t=a.value:e===`BANG_DEFAULT`?r=!0:e===`BANG_GLOBAL`&&(i=!0)}if(t&&n)if(r&&i){let e=this.variables;for(;e.parent!==null;)e=e.parent;e.get(t)===void 0&&this.variables.setGlobal(t,n)}else r?this.variables.get(t)===void 0&&this.variables.set(t,n):i?this.variables.setGlobal(t,n):this.variables.set(t,n)}_collectMixin(e){let t,n=[],r=new Map,i;for(let a of $(e))if(Q(a)){let e=a;if(e.ruleName===`mixin_params`){let t=this._extractParams(e);n=t.params,r=t.defaults}else e.ruleName===`block`&&(i=e)}else{let e=a,n=Xp(e);n===`FUNCTION`?t=e.value.replace(/\($/,``):n===`IDENT`&&(t=e.value)}t&&i&&this.mixins.set(t,{name:t,params:n,defaults:r,body:i})}_collectFunction(e){let t,n=[],r=new Map,i;for(let a of $(e))if(!Q(a))Xp(a)===`FUNCTION`&&(t=a.value.replace(/\($/,``));else{let e=a;if(e.ruleName===`mixin_params`){let t=this._extractParams(e);n=t.params,r=t.defaults}else e.ruleName===`function_body`&&(i=e)}t&&i&&this.functions.set(t,{name:t,params:n,defaults:r,body:i})}_extractParams(e){let t=[],n=new Map;for(let r of $(e)){if(!Q(r))continue;let e=r;if(e.ruleName===`mixin_param`){let r,i;for(let t of $(e))Q(t)?(t.ruleName===`value_list`||t.ruleName===`mixin_value_list`)&&(i=t):Xp(t)===`VARIABLE`&&(r=t.value);r&&(t.push(r),i!==void 0&&n.set(r,i))}}return{params:t,defaults:n}}_expandNode(e,t){if(!Q(e)){let n=e;return Xp(n)===`VARIABLE`?this._substituteVariable(n,t):n}let n=e;switch(n.ruleName){case`stylesheet`:return this._expandStylesheet(n,t);case`rule`:return this._expandTopLevelRule(n,t);case`lattice_rule`:return this._expandTopLevelLatticeRule(n,t);case`lattice_control`:return this._expandControl(n,t);case`block`:return this._expandBlock(n,t);case`block_contents`:return this._expandBlockContents(n,t);case`block_item`:return this._expandBlockItem(n,t);case`value_list`:return this._expandValueList(n,t);case`value`:return this._expandValue(n,t);case`function_call`:return this._expandFunctionCall(n,t);case`function_arg`:return this._expandChildren(n,t);case`function_args`:return this._expandChildren(n,t);case`compound_selector`:case`simple_selector`:case`class_selector`:return this._expandSelectorWithVars(n,t);default:return this._expandChildren(n,t)}}_expandTopLevelRule(e,t){let n=$(e);if(n.length===0)return e;let r=n[0];if(!Q(r))return this._expandChildren(e,t);let i=r;if(i.ruleName===`lattice_rule`){let n=this._expandTopLevelLatticeRule(i,t);return n===null?null:Array.isArray(n)?n:(cm(e,[n]),e)}return this._expandChildren(e,t)}_expandTopLevelLatticeRule(e,t){let n=$(e);if(n.length===0)return null;let r=n[0];if(!Q(r))return null;let i=r,a=i.ruleName;return a===`lattice_control`?this._expandControl(i,t):a===`variable_declaration`||a===`mixin_definition`||a===`function_definition`||a===`use_directive`?null:this._expandChildren(e,t)}_expandStylesheet(e,t){let n=[];for(let r of $(e)){let e=this._expandNode(r,t);e===null||(Array.isArray(e)?n.push(...e):n.push(e))}return cm(e,n),e}_expandChildren(e,t){let n=[];for(let r of $(e)){let e=this._expandNode(r,t);e!==null&&(Array.isArray(e)?n.push(...e):n.push(e))}return cm(e,n),e}_substituteVariable(e,t){let n=e.value,r=t.get(n);if(r===void 0)throw new kf(n,e.line??0,e.column??0);if(typeof r==`object`&&r&&`ruleName`in r){let n=sm(r),i=this._expandNode(n,t);return i===null?am(``,e):Array.isArray(i)?i[0]:i}return typeof r==`object`&&r&&`kind`in r?am($f(r),e):e}_expandBlock(e,t){let n=t.child();return this._expandChildren(e,n)}_expandBlockContents(e,t){let n=[];for(let r of $(e)){let e=this._expandBlockItemInner(r,t);e===null||(Array.isArray(e)?n.push(...e):n.push(e))}return cm(e,n),e}_expandBlockItemInner(e,t){if(!Q(e))return e;let n=e;if(n.ruleName===`block_item`){let e=$(n);if(e.length>0&&Q(e[0])){let r=e[0];if(r.ruleName===`lattice_block_item`){let e=this._expandLatticeBlockItem(r,t);return e===null?null:Array.isArray(e)?e:(cm(n,[r]),cm(r,[e]),n)}if(r.ruleName===`declaration_or_nested`){let e=$(r);if(e.length>0&&Q(e[0])&&e[0].ruleName===`property_nesting`){let n=this._expandPropertyNesting(e[0],t);return n.length>0?n:null}}}return this._expandChildren(n,t)}return this._expandChildren(n,t)}_expandBlockItem(e,t){let n=$(e);if(n.length===0)return e;let r=n[0];if(!Q(r))return this._expandChildren(e,t);let i=r;if(i.ruleName===`lattice_block_item`){let n=this._expandLatticeBlockItem(i,t);return n===null?null:Array.isArray(n)?n:(cm(e,[n]),e)}return this._expandChildren(e,t)}_expandLatticeBlockItem(e,t){let n=$(e);if(n.length===0)return e;let r=n[0];if(!Q(r))return e;let i=r,a=i.ruleName;return a===`variable_declaration`?(this._expandVariableDeclaration(i,t),null):a===`include_directive`?this._expandInclude(i,t):a===`lattice_control`?this._expandControl(i,t):a===`content_directive`?this._expandContent(t):a===`at_root_directive`?this._expandAtRoot(i,t):a===`extend_directive`?(this._collectExtend(i),null):this._expandChildren(e,t)}_expandVariableDeclaration(e,t){let n,r,i=!1,a=!1;for(let t of $(e))if(Q(t)){let e=t;if(e.ruleName===`value_list`)r=e;else if(e.ruleName===`variable_flag`){for(let t of $(e))if(!Q(t)){let e=Xp(t);e===`BANG_DEFAULT`?i=!0:e===`BANG_GLOBAL`&&(a=!0)}}}else{let e=Xp(t);e===`VARIABLE`?n=t.value:e===`BANG_DEFAULT`?i=!0:e===`BANG_GLOBAL`&&(a=!0)}if(n&&r){let e=this._expandNode(sm(r),t),o=e??r;try{let n=new Yp(t).evaluate(sm(e??r));n!=null&&(o=n)}catch{}if(i&&a){let e=t;for(;e.parent!==null;)e=e.parent;e.get(n)===void 0&&t.setGlobal(n,o)}else i?t.get(n)===void 0&&t.set(n,o):a?t.setGlobal(n,o):t.set(n,o)}}_expandValueList(e,t){let n=[];for(let r of $(e)){let e=this._expandNode(r,t);if(e!==null)if(Array.isArray(e))n.push(...e);else{let t=e;Q(t)&&t.ruleName===`value_list`?n.push(...$(t)):n.push(t)}}return cm(e,n),e}_expandValue(e,t){let n=$(e);if(n.length===0)return e;if(n.length===1&&!Q(n[0])){let r=n[0];if(Xp(r)===`VARIABLE`){let n=this._substituteVariable(r,t);return Q(n)&&n.ruleName===`value_list`?n:(cm(e,[n]),e)}}return this._expandChildren(e,t)}_expandFunctionCall(e,t){let n=$(e),r;for(let e of n)if(!Q(e)&&Xp(e)===`FUNCTION`){r=e.value.replace(/\($/,``);break}return r===void 0?this._expandChildren(e,t):this.functions.has(r)?this._evaluateFunctionCall(r,e,t):$p(r)&&!Kp.has(r)?this._expandChildren(e,t):Kp.has(r)?this._evaluateBuiltinFunction(r,e,t):($p(r),this._expandChildren(e,t))}_expandInclude(e,t){let n=$(e),r,i,a,o=null;for(let e of n)if(Q(e)){let t=e;t.ruleName===`include_args`?a=t:t.ruleName===`block`&&(o=t)}else{let t=e,n=Xp(t);n===`FUNCTION`?(r=t.value.replace(/\($/,``),i=t):n===`IDENT`&&(r=t.value,i=t)}if(r===void 0)return[];if(!this.mixins.has(r))throw new Af(r,i?.line??0,i?.column??0,[...this.mixins.keys()]);if(this.mixinStack.includes(r))throw new Mf(`mixin`,[...this.mixinStack,r]);let s=this.mixins.get(r),{positional:c,named:l}=a?this._parseIncludeArgs(a):{positional:[],named:new Map},u=c.length+l.size;if(u<s.params.length-s.defaults.size||u>s.params.length)throw new jf(`Mixin`,r,s.params.length,u);let d=e=>{let n=sm(e),r=this._expandNode(n,t);return r===null?e:Array.isArray(r)?r[0]??e:r},f=t.child(),p=0;for(let e=0;e<s.params.length;e++){let t=s.params[e];l.has(t)?f.set(t,d(l.get(t))):p<c.length?f.set(t,d(c[p++])):s.defaults.has(t)&&f.set(t,sm(s.defaults.get(t)))}this.contentBlockStack.push(o),this.contentScopeStack.push(t),this.mixinStack.push(r);try{let e=sm(s.body),t=this._expandNode(e,f),n=Array.isArray(t)?t[0]:t;if(n&&Q(n)){for(let e of $(n))if(Q(e)&&e.ruleName===`block_contents`)return $(e).filter(e=>e!==null)}return[]}finally{this.mixinStack.pop(),this.contentBlockStack.pop(),this.contentScopeStack.pop()}}_parseIncludeArgs(e){let t=[],n=new Map;for(let r of $(e)){if(!Q(r))continue;let e=r;if(e.ruleName===`include_arg`){let r=$(e);if(r.length>=3&&!Q(r[0])&&Xp(r[0])===`VARIABLE`&&!Q(r[1])&&Xp(r[1])===`COLON`){let e=r[0].value,t=r[2];n.set(e,t)}else{let e=r.find(e=>Q(e)&&e.ruleName===`value_list`);e&&t.push(e)}}else e.ruleName===`value_list`&&t.push(e)}if(t.length===1&&n.size===0){let e=this._splitValueListOnCommas(t[0]);if(e.length>1)return{positional:e,named:n}}return{positional:t,named:n}}_splitValueListOnCommas(e){let t=$(e),n=!1;for(let e of t)if(Q(e)&&e.ruleName===`value`){for(let t of $(e))if(!Q(t)&&Xp(t)===`COMMA`){n=!0;break}}if(!n)return[e];let r=[[]];for(let e of t){if(Q(e)&&e.ruleName===`value`){let t=$(e);if(t.length===1&&!Q(t[0])&&Xp(t[0])===`COMMA`){r.push([]);continue}}r[r.length-1].push(e)}return r.filter(e=>e.length>0).map(e=>new tm(`value_list`,e))}_expandControl(e,t){let n=$(e);if(n.length===0)return null;let r=n[0];if(!Q(r))return null;let i=r;switch(i.ruleName){case`if_directive`:return this._expandIf(i,t);case`for_directive`:return this._expandFor(i,t);case`each_directive`:return this._expandEach(i,t);case`while_directive`:return this._expandWhile(i,t)}return null}_expandIf(e,t){let n=$(e),r=[],i=0;for(;i<n.length;){let e=n[i],t=Zp(e);if(t===`@if`){let e=n[i+1],t=n[i+2];t&&Q(t)&&r.push({condition:e,block:t}),i+=3}else if(t===`@else`)if(i+1<n.length&&Zp(n[i+1])===`if`){let e=n[i+2],t=n[i+3];t&&Q(t)&&r.push({condition:e,block:t}),i+=4}else{let e=n[i+1];e&&Q(e)&&r.push({condition:null,block:e}),i+=2}else i++}let a=new Yp(t);for(let{condition:e,block:n}of r)if(e===null)return this._expandBlockToItems(n,t);else if(Xf(a.evaluate(e)))return this._expandBlockToItems(n,t);return[]}_expandFor(e,t){let n=$(e),r,i,a,o=!1,s,c=0;for(;c<n.length;){let e=n[c],t=Zp(e);t!==void 0&&!Q(e)&&Xp(e)===`VARIABLE`?r=t:t===`from`?(i=n[c+1],c++):t===`through`?(o=!0,a=n[c+1],c++):t===`to`?(o=!1,a=n[c+1],c++):Q(e)&&e.ruleName===`block`&&(s=e),c++}if(!r||!i||!a||!s)return[];let l=new Yp(t),u=l.evaluate(i),d=l.evaluate(a),f=u.kind===`number`?Math.trunc(u.value):0,p=d.kind===`number`?Math.trunc(d.value):0,m=o?p+1:p,h=[];for(let e=f;e<m;e++){let n=t.child();n.set(r,new Bf(e));let i=this._expandBlockToItems(sm(s),n);h.push(...i)}return h}_expandEach(e,t){let n=$(e),r=[],i,a;for(let e of n)if(Q(e)){let t=e;t.ruleName===`each_list`?i=t:t.ruleName===`block`&&(a=t)}else{let t=e;Xp(t)===`VARIABLE`&&r.push(t.value)}if(r.length===0||!i||!a)return[];let o=this._resolveEachList(i,t);if(o!==null)return this._expandEachOverResolved(r,o,a,t);let s=[];for(let e of $(i))Q(e)&&e.ruleName===`value`&&s.push(e);let c=[];for(let e of s){let n=t.child();if(r.length>0){let t=this._extractValueToken(e);n.set(r[0],t)}let i=this._expandBlockToItems(sm(a),n);c.push(...i)}return c}_resolveEachList(e,t){let n=[];for(let t of $(e))if(Q(t)&&t.ruleName===`value`)for(let e of $(t))!Q(e)&&Xp(e)===`VARIABLE`&&n.push(e);if(n.length===1){let e=t.get(n[0].value);if(typeof e==`object`&&e&&`kind`in e){let t=e;if(t.kind===`map`||t.kind===`list`)return t}if(typeof e==`object`&&e&&`ruleName`in e){let n=this._findMapLiteralInAst(e);if(n)return this._convertMapLiteralToLatticeMap(n,t)}}return null}_findMapLiteralInAst(e){if(e.ruleName===`map_literal`)return e;for(let t of $(e))if(Q(t)){let e=this._findMapLiteralInAst(t);if(e)return e}return null}_convertMapLiteralToLatticeMap(e,t){let n=[],r=new Yp(t);for(let t of $(e)){if(!Q(t)||t.ruleName!==`map_entry`)continue;let e,i;for(let n of $(t))if(Q(n)){let e=n;e.ruleName===`lattice_expression`&&i===void 0&&(i=e)}else{let t=n,r=Xp(t);(r===`IDENT`||r===`STRING`)&&e===void 0&&(e=t.value.replace(/^"|"$/g,``).replace(/^'|'$/g,``))}if(e!==void 0&&i!==void 0){let t=r.evaluate(i);n.push([e,t])}}return new Yf(n)}_expandEachOverResolved(e,t,n,r){let i=[];if(t.kind===`map`)for(let[a,o]of t.items){let t=r.child();t.set(e[0],new Wf(a)),e.length>=2&&t.set(e[1],o),i.push(...this._expandBlockToItems(sm(n),t))}else if(t.kind===`list`)for(let a of t.items){let t=r.child();t.set(e[0],a),i.push(...this._expandBlockToItems(sm(n),t))}return i}_extractValueToken(e){if(Q(e)){let t=$(e);if(t.length===1&&!Q(t[0]))return Zf(t[0])}return e}_expandBlockToItems(e,t){let n=this._expandNode(e,t),r=Array.isArray(n)?n[0]:n;if(r&&Q(r)){for(let e of $(r))if(Q(e)&&e.ruleName===`block_contents`)return $(e).filter(e=>e!==null)}return[]}_evaluateFunctionCall(e,t,n){let r=this.functions.get(e),i=$(t),a=[];for(let e of i)if(Q(e)&&e.ruleName===`function_args`){a=this._parseFunctionCallArgs(e,n);break}let o=r.params.length-r.defaults.size;if(a.length<o||a.length>r.params.length)throw new jf(`Function`,e,r.params.length,a.length);if(this.functionStack.includes(e))throw new Mf(`function`,[...this.functionStack,e]);let s=this.variables.child();for(let e=0;e<r.params.length;e++){let t=r.params[e];e<a.length?s.set(t,a[e]):r.defaults.has(t)&&s.set(t,sm(r.defaults.get(t)))}this.functionStack.push(e);try{let n=sm(r.body);try{this._evaluateFunctionBody(n,s)}catch(e){if(e instanceof em)return om($f(e.value),t);throw e}throw new Nf(e)}finally{this.functionStack.pop()}}_evaluateFunctionBody(e,t){if(Q(e))for(let n of $(e)){if(!Q(n))continue;let e=n;if(e.ruleName===`function_body_item`){let n=$(e);if(n.length===0)continue;let r=n[0];if(!Q(r))continue;let i=r;i.ruleName===`variable_declaration`?this._expandVariableDeclaration(i,t):i.ruleName===`return_directive`?this._evaluateReturn(i,t):i.ruleName===`lattice_control`&&this._evaluateControlInFunction(i,t)}else this._evaluateFunctionBody(e,t)}}_evaluateReturn(e,t){for(let n of $(e))if(Q(n)&&n.ruleName===`lattice_expression`)throw new em(new Yp(t).evaluate(n));throw new em(new qf)}_evaluateControlInFunction(e,t){let n=$(e);if(n.length===0)return;let r=n[0];if(!Q(r))return;let i=r;i.ruleName===`if_directive`&&this._evaluateIfInFunction(i,t)}_evaluateIfInFunction(e,t){let n=$(e),r=[],i=0;for(;i<n.length;){let e=n[i],t=Zp(e);if(t===`@if`){let e=n[i+1],t=n[i+2];t&&Q(t)&&r.push({condition:e,block:t}),i+=3}else if(t===`@else`)if(i+1<n.length&&Zp(n[i+1])===`if`){let e=n[i+2],t=n[i+3];t&&Q(t)&&r.push({condition:e,block:t}),i+=4}else{let e=n[i+1];e&&Q(e)&&r.push({condition:null,block:e}),i+=2}else i++}let a=new Yp(t);for(let{condition:e,block:n}of r)if(e===null||Xf(a.evaluate(e))){this._evaluateBlockInFunction(n,t);return}}_evaluateBlockInFunction(e,t){if(Q(e))for(let n of $(e)){if(!Q(n))continue;let e=n;if(e.ruleName===`block_contents`)this._evaluateBlockInFunction(e,t);else if(e.ruleName===`block_item`){let n=$(e);if(n.length>0&&Q(n[0])){let e=n[0];if(e.ruleName===`at_rule`)this._maybeEvaluateReturnAtRule(e,t);else if(e.ruleName===`lattice_block_item`)for(let n of $(e))Q(n)&&n.ruleName===`variable_declaration`&&this._expandVariableDeclaration(n,t)}}}}_maybeEvaluateReturnAtRule(e,t){let n,r;for(let t of $(e))Q(t)?t.ruleName===`at_prelude`&&(r=t):Xp(t)===`AT_KEYWORD`&&(n=t.value);if(n!==`@return`||!r)return;let i=[];if(this._collectTokens(r,i),i.length===0)throw new em(new qf);if(i.length===1){let e=i[0];if(Xp(e)===`VARIABLE`){let n=t.get(e.value);if(n!==void 0){if(typeof n==`object`&&n&&`kind`in n)throw new em(n);if(typeof n==`object`&&n&&`ruleName`in n)throw new em(Qf(n))}}throw new em(Zf(e))}throw new em(Zf(i[0]))}_collectTokens(e,t){if(!Q(e)){t.push(e);return}for(let n of $(e))this._collectTokens(n,t)}_parseFunctionCallArgs(e,t){let n=[[]];for(let r of $(e)){if(!Q(r)&&Xp(r)===`COMMA`){n.push([]);continue}if(Q(r)&&r.ruleName===`function_arg`)for(let e of $(r))if(Q(e))n[n.length-1].push(e);else{if(Xp(e)===`COMMA`){n.push([]);continue}let r=e;if(t&&Xp(r)===`VARIABLE`){let e=t.get(r.value);e==null?n[n.length-1].push(Zf(r)):typeof e==`object`&&`kind`in e?n[n.length-1].push(e):typeof e==`object`&&`type`in e?n[n.length-1].push(Zf(e)):typeof e==`object`&&`ruleName`in e?n[n.length-1].push(Qf(e)):n[n.length-1].push(Zf(r))}else n[n.length-1].push(Zf(r))}}let r=[];for(let e of n)(e.length===1||e.length>1)&&r.push(e[0]);return r}_expandWhile(e,t){let n=$(e),r,i;for(let e of n)if(Q(e)){let t=e;t.ruleName===`lattice_expression`?r=t:t.ruleName===`block`&&(i=t)}if(!r||!i)return[];let a=[],o=0;for(;Xf(new Yp(t).evaluate(sm(r)));){if(o++,o>this.maxWhileIterations)throw new Pf(this.maxWhileIterations);let e=this._expandBlockToItems(sm(i),t);a.push(...e)}return a}_expandSelectorWithVars(e,t){let n=[];for(let r of $(e))if(Q(r)){let e=this._expandNode(r,t);e!==null&&(Array.isArray(e)?n.push(...e):n.push(e))}else{let e=r;if(Xp(e)===`VARIABLE`){let r=e.value,i=t.get(r);if(i===void 0)throw new kf(r,e.line??0,e.column??0);let a;a=typeof i==`object`&&i&&`kind`in i?$f(i):typeof i==`object`&&i&&`ruleName`in i?$f(Qf(i)):String(i),a=a.replace(/^"|"$/g,``).replace(/^'|'$/g,``),n.push(am(a,e))}else n.push(r)}return cm(e,n),e}_expandContent(e){if(this.contentBlockStack.length===0)return[];let t=this.contentBlockStack[this.contentBlockStack.length-1];if(t===null)return[];let n=this.contentScopeStack.length>0?this.contentScopeStack[this.contentScopeStack.length-1]:e;return this._expandBlockToItems(sm(t),n)}_expandAtRoot(e,t){let n=$(e),r,i;for(let e of n)if(Q(e)){let t=e;t.ruleName===`block`?r=t:t.ruleName===`selector_list`&&(i=t)}if(!r)return null;if(i){let e=new tm(`qualified_rule`,[this._expandNode(sm(i),t),this._expandNode(sm(r),t)]);this.atRootRules.push(e)}else{let e=this._expandBlockToItems(sm(r),t);this.atRootRules.push(...e)}return null}_collectExtend(e){let t=``;for(let n of $(e))if(Q(n)&&n.ruleName===`extend_target`){let e=[];for(let t of $(n))Q(t)||e.push(t.value);t=e.join(``)}t&&(this.extendMap.has(t)||this.extendMap.set(t,[]))}_expandPropertyNesting(e,t){let n=``,r;for(let t of $(e))if(Q(t)){let e=t;if(e.ruleName===`property`)for(let t of $(e))Q(t)||(n=t.value);else e.ruleName===`block`&&(r=e)}if(!n||!r)return[];let i=this._expandNode(sm(r),t),a=[];return this._flattenNestedProps(i,n,a),a}_flattenNestedProps(e,t,n){if(Q(e))for(let r of $(e)){if(!Q(r))continue;let e=r;e.ruleName===`block_contents`?this._flattenNestedProps(e,t,n):e.ruleName===`block_item`?this._flattenNestedBlockItem(e,t,n):e.ruleName===`declaration`&&this._rewriteDeclarationPrefix(e,t,n)}}_flattenNestedBlockItem(e,t,n){let r=$(e);if(r.length===0)return;let i=r[0];if(!Q(i))return;let a=i;if(a.ruleName===`declaration_or_nested`){for(let e of $(a))if(Q(e)){let r=e;if(r.ruleName===`declaration`)this._rewriteDeclarationPrefix(r,t,n);else if(r.ruleName===`property_nesting`){let e=this._expandPropertyNestingWithPrefix(r,t);n.push(...e)}}}}_rewriteDeclarationPrefix(e,t,n){for(let n of $(e))if(Q(n)&&n.ruleName===`property`){for(let e of $(n))if(!Q(e)){let n=e;n.value=`${t}-${n.value}`}}n.push(e)}_expandPropertyNestingWithPrefix(e,t){let n=``,r;for(let t of $(e))if(Q(t)){let e=t;if(e.ruleName===`property`)for(let t of $(e))Q(t)||(n=t.value);else e.ruleName===`block`&&(r=e)}let i=`${t}-${n}`,a=[];return r&&this._flattenNestedProps(r,i,a),a}_evaluateBuiltinFunction(e,t,n){let r=$(t),i=[];for(let e of r)if(Q(e)&&e.ruleName===`function_args`){let t=new Yp(n);i=this._collectBuiltinFunctionArgs(e,t);break}let a=Kp.get(e)(i);return a.kind===`null`?this._expandChildren(t,n):om($f(a),t)}_collectBuiltinFunctionArgs(e,t){let n=[],r=[],i=()=>{if(r.length>0){if(r.length===1){let e=r[0];if(Xp(e)===`VARIABLE`){let r=t.scope.get(e.value);typeof r==`object`&&r&&`kind`in r?n.push(r):typeof r==`object`&&r&&`ruleName`in r?n.push(Qf(r)):n.push(Zf(e))}else n.push(Zf(e))}else n.push(Zf(r[0]));r.length=0}};for(let a of $(e)){if(!Q(a)&&Xp(a)===`COMMA`){i();continue}if(Q(a)&&a.ruleName===`function_arg`)for(let e of $(a))if(Q(e))n.push(t.evaluate(e)),r.length=0;else{if(Xp(e)===`COMMA`){i();continue}r.push(e)}}return i(),n}_applyExtends(e){this._removePlaceholderRules(e)}_removePlaceholderRules(e){if(!Q(e))return;let t=[];for(let n of $(e))n!==null&&(this._isPlaceholderOnlyRule(n)||(this._removePlaceholderRules(n),t.push(n)));cm(e,t)}_isPlaceholderOnlyRule(e){if(!Q(e))return!1;let t=e;if(t.ruleName===`qualified_rule`){let e=this._extractSelectorText(t).split(`,`).map(e=>e.trim()).filter(e=>e);return e.length>0&&e.every(e=>e.startsWith(`%`))}if(t.ruleName===`rule`){let e=$(t);if(e.length>0&&Q(e[0]))return this._isPlaceholderOnlyRule(e[0])}return!1}_extractSelectorText(e){for(let t of $(e))if(Q(t)&&t.ruleName===`selector_list`)return this._collectText(t);return``}_collectText(e){if(!Q(e))return e.value;let t=[];for(let n of $(e))t.push(this._collectText(n));return t.join(` `)}_spliceAtRootRules(e){for(let t of this.atRootRules)t!==null&&$(e).push(t)}_cleanup(e){if(!Q(e))return e;let t=e,n=[];for(let e of $(t)){if(e===null)continue;let t=this._cleanup(e);t!==null&&n.push(t)}return cm(t,n),t}};function um(e){return`ruleName`in e}function dm(e){return e.type}var fm=class{indent;minified;constructor(e=`  `,t=!1){this.indent=e,this.minified=t}emit(e){let t=this._emitNode(e,0).trim();return t?t+`
`:``}_emitNode(e,t){if(!um(e))return e.value;let n=e;switch(n.ruleName){case`stylesheet`:return this._emitStylesheet(n,t);case`rule`:return this._emitRule(n,t);case`qualified_rule`:return this._emitQualifiedRule(n,t);case`at_rule`:return this._emitAtRule(n,t);case`at_prelude`:return this._emitAtPrelude(n,t);case`at_prelude_token`:return this._emitDefault(n,t);case`at_prelude_tokens`:return this._emitAtPreludeTokens(n,t);case`function_in_prelude`:return this._emitFunctionInPrelude(n,t);case`paren_block`:return this._emitParenBlock(n,t);case`selector_list`:return this._emitSelectorList(n,t);case`complex_selector`:return this._emitComplexSelector(n,t);case`combinator`:return this._emitCombinator(n,t);case`compound_selector`:return this._emitCompoundSelector(n,t);case`simple_selector`:return this._emitSimpleSelector(n,t);case`subclass_selector`:return this._emitSubclassSelector(n,t);case`class_selector`:return this._emitClassSelector(n,t);case`id_selector`:return this._emitIdSelector(n,t);case`attribute_selector`:return this._emitAttributeSelector(n,t);case`attr_matcher`:return this._emitAttrMatcher(n,t);case`attr_value`:return this._emitAttrValue(n,t);case`pseudo_class`:return this._emitPseudoClass(n,t);case`pseudo_class_args`:return this._emitPseudoClassArgs(n,t);case`pseudo_class_arg`:return this._emitDefault(n,t);case`pseudo_element`:return this._emitPseudoElement(n,t);case`block`:return this._emitBlock(n,t);case`block_contents`:return this._emitBlockContents(n,t);case`block_item`:return this._emitBlockItem(n,t);case`declaration_or_nested`:return this._emitDeclarationOrNested(n,t);case`declaration`:return this._emitDeclaration(n,t);case`property`:return this._emitProperty(n,t);case`priority`:return this._emitPriority(n,t);case`value_list`:return this._emitValueList(n,t);case`value`:return this._emitValue(n,t);case`function_call`:return this._emitFunctionCall(n,t);case`function_args`:return this._emitFunctionArgs(n,t);case`function_arg`:return this._emitFunctionArg(n,t);default:return this._emitDefault(n,t)}}_emitStylesheet(e,t){let n=[];for(let r of e.children){let e=this._emitNode(r,t);e.trim()&&n.push(e)}return this.minified?n.join(``):n.join(`

`)}_emitRule(e,t){let n=e.children;return n.length>0?this._emitNode(n[0],t):``}_emitQualifiedRule(e,t){let n=``,r=``;for(let i of e.children){if(!um(i))continue;let e=i;if(e.ruleName===`selector_list`)n=this._emitNode(i,t);else if(e.ruleName===`block`)r=this._emitBlock(e,t);else{let e=this._emitNode(i,t);e.trim()&&(n+=e)}}return this.minified?`${n}${r}`:n?`${n} ${r}`:r}_emitAtRule(e,t){let n=``,r=``,i=``,a=!1;for(let o of e.children)if(um(o)){let e=o;e.ruleName===`at_prelude`?r=this._emitAtPrelude(e,t):e.ruleName===`block`&&(i=this._emitBlock(e,t))}else{let e=o,t=dm(e);t===`AT_KEYWORD`?n=e.value:t===`SEMICOLON`&&(a=!0)}if(this.minified)return a?`${n}${r};`:`${n}${r}${i}`;if(a){let e=r.trim()?` ${r.trim()}`:``;return`${n}${e};`}let o=r.trim()?` ${r.trim()}`:``;return`${n}${o} ${i}`}_emitAtPrelude(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}_emitAtPreludeTokens(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}_emitFunctionInPrelude(e,t){let n=[];for(let r of e.children)if(um(r))n.push(this._emitNode(r,t));else{let e=r;dm(e)===`RPAREN`?n.push(`)`):n.push(e.value)}return n.join(``)}_emitParenBlock(e,t){let n=[];for(let r of e.children)if(um(r))n.push(this._emitNode(r,t));else{let e=r,t=dm(e);t===`LPAREN`?n.push(`(`):t===`RPAREN`?n.push(`)`):n.push(e.value)}return n.join(``)}_emitSelectorList(e,t){let n=[];for(let r of e.children)um(r)&&n.push(this._emitNode(r,t));let r=this.minified?`,`:`, `;return n.join(r)}_emitComplexSelector(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}_emitCombinator(e,t){return e.children.length>0?e.children[0].value:``}_emitCompoundSelector(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(``)}_emitSimpleSelector(e,t){return e.children.length>0?e.children[0].value:``}_emitSubclassSelector(e,t){return e.children.length>0?this._emitNode(e.children[0],t):``}_emitClassSelector(e,t){let n=[];for(let t of e.children)um(t)||n.push(t.value);return n.join(``)}_emitIdSelector(e,t){return e.children.length>0?e.children[0].value:``}_emitAttributeSelector(e,t){let n=[];for(let r of e.children)if(um(r))n.push(this._emitNode(r,t));else{let e=r,t=dm(e);t===`LBRACKET`?n.push(`[`):t===`RBRACKET`?n.push(`]`):n.push(e.value)}return n.join(``)}_emitAttrMatcher(e,t){return e.children.length>0?e.children[0].value:``}_emitAttrValue(e,t){if(e.children.length>0){let t=e.children[0];return dm(t)===`STRING`?`"${t.value}"`:t.value}return``}_emitPseudoClass(e,t){let n=[];for(let r of e.children)if(um(r))n.push(this._emitNode(r,t));else{let e=r,t=dm(e);t===`COLON`?n.push(`:`):t===`RPAREN`?n.push(`)`):n.push(e.value)}return n.join(``)}_emitPseudoClassArgs(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(``)}_emitPseudoElement(e,t){let n=[];for(let t of e.children)if(!um(t)){let e=t;dm(e)===`COLON_COLON`?n.push(`::`):n.push(e.value)}return n.join(``)}_emitBlock(e,t){let n;for(let t of e.children)if(um(t)&&t.ruleName===`block_contents`){n=t;break}if(this.minified)return n?`{`+this._emitBlockContents(n,t+1)+`}`:`{}`;if(!n)return`{
`+this.indent.repeat(t)+`}`;let r=this._emitBlockContents(n,t+1);return r.trim()?`{
`+r+`
`+this.indent.repeat(t)+`}`:`{
`+this.indent.repeat(t)+`}`}_emitBlockContents(e,t){let n=[];for(let r of e.children){let e=this._emitNode(r,t);e.trim()&&n.push(e)}if(this.minified)return n.join(``);let r=this.indent.repeat(t);return n.map(e=>`${r}${e}`).join(`
`)}_emitBlockItem(e,t){return e.children.length>0?this._emitNode(e.children[0],t):``}_emitDeclarationOrNested(e,t){return e.children.length>0?this._emitNode(e.children[0],t):``}_emitDeclaration(e,t){let n=``,r=``,i=``;for(let t of e.children){if(!um(t))continue;let e=t;e.ruleName===`property`?n=this._emitProperty(e,0):e.ruleName===`value_list`?r=this._emitValueList(e,0):e.ruleName===`priority`&&(i=` !important`)}return this.minified?`${n}:${r}${i};`:`${n}: ${r}${i};`}_emitProperty(e,t){return e.children.length>0?e.children[0].value:``}_emitPriority(e,t){return`!important`}_emitValueList(e,t){let n=[];for(let r of e.children){let e=this._emitNode(r,t);n.push(e)}let r=n.join(` `);return r=r.replace(/ , /g,`, `).replace(/ ,/g,`,`),r}_emitValue(e,t){let n=e.children;if(n.length===1){let e=n[0];if(!um(e)){let t=e;return dm(t)===`STRING`?`"${t.value}"`:t.value}return this._emitNode(e,t)}return this._emitDefault(e,t)}_emitFunctionCall(e,t){let n=e.children;if(n.length===1)return n[0].value;let r=[];for(let e of n)if(um(e))r.push(this._emitNode(e,t));else{let t=e,n=dm(t);n===`FUNCTION`?r.push(t.value):n===`RPAREN`?r.push(`)`):r.push(t.value)}return r.join(``)}_emitFunctionArgs(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));let r=n.join(` `);return r=r.replace(/ , /g,`, `).replace(/ ,/g,`,`),r}_emitFunctionArg(e,t){let n=e.children;if(n.length===1){let e=n[0];return um(e)?this._emitNode(e,t):e.value}let r=[];for(let e of n)if(um(e))r.push(this._emitNode(e,t));else{let t=e;r.push(dm(t)===`RPAREN`?`)`:t.value)}return r.join(``)}_emitDefault(e,t){let n=[];for(let r of e.children)n.push(this._emitNode(r,t));return n.join(` `)}};function pm(e,t={}){let n=Tf(e),r=new lm().transform(n);return new fm(t.indent??`  `,t.minified??!1).emit(r)}function mm(e,t={}){return pm(e,t)}var hm=`$paper: #f7f8f3;
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

.workspace--precision-residency {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  align-items: start;
  gap: 18px;
}

.precision-stage {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.precision-intro,
.precision-paper,
.precision-formats,
.precision-trace,
.precision-residency,
.precision-controls {
  min-width: 0;
  border: 1px solid $line;
  border-radius: 12px;
  background: $paper;
  box-shadow: 0 14px 35px rgba(23, 32, 28, 0.08);
}

.precision-intro,
.precision-paper,
.precision-formats,
.precision-trace,
.precision-residency {
  padding: 16px;
}

.precision-intro {
  display: flex;
  justify-content: space-between;
  gap: 16px;
}

.precision-intro h2,
.precision-intro p,
.precision-controls h2,
.precision-controls p {
  margin: 0;
}

.precision-intro p:not(.eyebrow),
.precision-controls > p:not(.eyebrow),
.precision-controls section p:not(.eyebrow) {
  color: $muted;
  line-height: 1.45;
}

.precision-chip {
  align-self: start;
  border-radius: 999px;
  background: rgba(109, 91, 208, 0.1);
  color: #5543b8;
  padding: 7px 10px;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 0.7rem;
  white-space: nowrap;
}

.precision-paper .panel-heading,
.precision-formats .panel-heading,
.precision-trace .panel-heading,
.precision-residency .panel-heading {
  align-items: start;
}

.precision-paper .panel-heading > code,
.precision-formats .panel-heading > code,
.precision-residency .panel-heading > code {
  border-radius: 7px;
  background: rgba(37, 99, 235, 0.06);
  padding: 6px 8px;
  color: #234c9f;
}

.precision-equation-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.precision-equation-row code {
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.06);
  padding: 13px;
  text-align: center;
}

.precision-equation-row strong {
  color: #1d6849;
}

.precision-format-buttons {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 9px;
}

.precision-format-buttons button,
.precision-strategy-buttons button {
  display: grid;
  gap: 5px;
  min-width: 0;
  border: 1px solid $line;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.85);
  color: $ink;
  padding: 11px;
  text-align: left;
}

.precision-format-buttons button[aria-pressed="true"],
.precision-strategy-buttons button[aria-pressed="true"] {
  border-color: rgba(109, 91, 208, 0.5);
  background: rgba(109, 91, 208, 0.09);
}

.precision-format-buttons small,
.precision-strategy-buttons span,
.precision-transfer-flow small {
  color: $muted;
  font-size: 0.68rem;
}

.precision-format-buttons code {
  overflow: hidden;
  color: #234c9f;
  text-overflow: ellipsis;
}

.precision-scale-note {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
  margin: 11px 0 0;
  border-radius: 9px;
  background: rgba(183, 121, 31, 0.07);
  padding: 10px;
}

.precision-scale-note span {
  color: $muted;
}

.precision-table {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  overflow-x: auto;
}

.precision-table > div {
  display: grid;
  grid-template-columns: repeat(6, minmax(112px, 1fr));
  min-width: 700px;
  border-top: 1px solid $line;
}

.precision-table > div:first-child {
  border-top: 0;
}

.precision-table code,
.precision-table strong {
  padding: 9px;
  overflow-wrap: anywhere;
}

.precision-table-head {
  color: $muted;
  font-size: 0.7rem;
}

.precision-transfer-flow {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 8px;
}

.precision-transfer-flow > div {
  display: grid;
  gap: 5px;
  border-radius: 9px;
  background: rgba(37, 99, 235, 0.055);
  padding: 11px;
  text-align: center;
}

.precision-transfer-flow > span {
  color: $muted;
}

.precision-transfer-equation {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 10px;
  border-radius: 9px;
  background: rgba(35, 122, 87, 0.06);
  padding: 10px;
}

.precision-transfer-equation strong {
  color: #1d6849;
}

.precision-controls {
  position: sticky;
  top: 18px;
  display: grid;
  gap: 12px;
  padding: 16px;
}

.precision-controls label {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
  color: $muted;
  font-size: 0.75rem;
}

.precision-controls label input {
  grid-column: 1 / -1;
  width: 100%;
}

.precision-strategy-buttons {
  display: grid;
  gap: 7px;
}

.precision-controls section {
  border-top: 1px solid $line;
  padding-top: 12px;
}

.precision-controls ol {
  margin: 8px 0 0;
  padding-left: 20px;
  color: $muted;
  line-height: 1.5;
}

.precision-warning {
  border-radius: 9px;
  background: rgba(183, 121, 31, 0.07);
  padding: 11px;
}

@media (max-width: 1180px) {
  .workspace--precision-residency {
    grid-template-columns: 1fr;
  }

  .precision-controls {
    position: static;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .precision-controls > p,
  .precision-controls > h2,
  .precision-controls > label,
  .precision-strategy-buttons {
    grid-column: 1 / -1;
  }
}

@media (max-width: 820px) {
  .precision-intro,
  .precision-paper .panel-heading,
  .precision-formats .panel-heading,
  .precision-trace .panel-heading,
  .precision-residency .panel-heading {
    display: grid;
  }

  .precision-chip {
    justify-self: start;
  }

  .precision-equation-row,
  .precision-format-buttons,
  .precision-transfer-flow,
  .precision-controls {
    grid-template-columns: 1fr;
  }

  .precision-transfer-flow > span {
    transform: rotate(90deg);
    justify-self: center;
  }

  .precision-controls > p,
  .precision-controls > h2,
  .precision-controls > label,
  .precision-strategy-buttons {
    grid-column: auto;
  }
}
`,gm=`coding-adventures-lattice-styles`;function _m(){if(document.getElementById(gm)===null)try{let e=document.createElement(`style`);e.id=gm,e.textContent=mm(hm),document.head.append(e)}catch(e){console.error(`Failed to install Lattice styles`,e)}}_m(),(0,u.createRoot)(document.getElementById(`root`)).render((0,E.jsx)(l.StrictMode,{children:(0,E.jsx)(rf,{})}));