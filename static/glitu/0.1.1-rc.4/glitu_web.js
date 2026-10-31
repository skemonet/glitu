/**
 * Options passed by the page: settings of the theme (glitu.json), its name and its logo
 * @param {any} options
 */
export function start(options) {
    wasm.start(options);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_boolean_get_5b446f51afd21013: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_4687d8d8c2017d52: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_undefined_8865fb403f8fe9d8: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_2e0e7dee9f701a71: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_0380ccaa2f57f0d9: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_41e9ee4f547fc59a: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_dcc1a90847f04c41: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_addColorStop_2fe19c9df420d0a6: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addColorStop(arg1, getStringFromWasm0(arg2, arg3));
        }, arguments); },
        __wbg_addEventListener_4d0db17c671ea324: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_addEventListener_8d3235978b9c69a8: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_add_9ceab44da1353b80: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.add(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_altKey_1e5fe17a42050103: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_altKey_cfb4c4fab9a7d0d0: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_appendChild_fb8c52e7dd8484ea: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_arc_03f7717b2e38682a: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.arc(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_atan2_70e726ccd08a4fcf: function(arg0, arg1) {
            const ret = Math.atan2(arg0, arg1);
            return ret;
        },
        __wbg_baseURI_8f55f952617f648f: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.baseURI;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_beginPath_8598d895c13f1c86: function(arg0) {
            arg0.beginPath();
        },
        __wbg_blur_cb8bd1ead8743f99: function() { return handleError(function (arg0) {
            arg0.blur();
        }, arguments); },
        __wbg_body_e549239eaff082e1: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_button_1aa380c7ca420cd1: function(arg0) {
            const ret = arg0.button;
            return ret;
        },
        __wbg_cancelAnimationFrame_6d30ac112c49342a: function() { return handleError(function (arg0, arg1) {
            arg0.cancelAnimationFrame(arg1);
        }, arguments); },
        __wbg_changedTouches_df984754af5ef0fa: function(arg0) {
            const ret = arg0.changedTouches;
            return ret;
        },
        __wbg_childNodes_9542af79c583ae90: function(arg0) {
            const ret = arg0.childNodes;
            return ret;
        },
        __wbg_classList_8a097a4837412bf6: function(arg0) {
            const ret = arg0.classList;
            return ret;
        },
        __wbg_clearRect_a5e36505218c247a: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearRect(arg1, arg2, arg3, arg4);
        },
        __wbg_clearTimeout_c7bc4c3c7a7774af: function(arg0, arg1) {
            arg0.clearTimeout(arg1);
        },
        __wbg_clientHeight_33dd2a2c8630a1f1: function(arg0) {
            const ret = arg0.clientHeight;
            return ret;
        },
        __wbg_clientWidth_08d5512595aacb73: function(arg0) {
            const ret = arg0.clientWidth;
            return ret;
        },
        __wbg_clientX_5075231ed45e8ef5: function(arg0) {
            const ret = arg0.clientX;
            return ret;
        },
        __wbg_clientX_80fda6c64299fd4d: function(arg0) {
            const ret = arg0.clientX;
            return ret;
        },
        __wbg_clientY_0206292d4e8f94cb: function(arg0) {
            const ret = arg0.clientY;
            return ret;
        },
        __wbg_clientY_846c39814ce02ab2: function(arg0) {
            const ret = arg0.clientY;
            return ret;
        },
        __wbg_cloneNode_a3ed8db010540e0d: function() { return handleError(function (arg0) {
            const ret = arg0.cloneNode();
            return ret;
        }, arguments); },
        __wbg_closePath_355eb6da179dd9c2: function(arg0) {
            arg0.closePath();
        },
        __wbg_closed_0789702fcb586b8a: function() { return handleError(function (arg0) {
            const ret = arg0.closed;
            return ret;
        }, arguments); },
        __wbg_closest_6abd1bf5d7559cda: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.closest(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_contains_05d0776afca2789a: function(arg0, arg1, arg2) {
            const ret = arg0.contains(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_contains_dd3796baeaeb3211: function(arg0, arg1) {
            const ret = arg0.contains(arg1);
            return ret;
        },
        __wbg_contentWindow_f8cbf28db0e23c34: function(arg0) {
            const ret = arg0.contentWindow;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_cos_640f0df62a857f39: function(arg0) {
            const ret = Math.cos(arg0);
            return ret;
        },
        __wbg_createElementNS_957ec8b66176901e: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createElementNS(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return ret;
        }, arguments); },
        __wbg_createElement_74049073a11f9c31: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createLinearGradient_d67a7d18d11c0b62: function(arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createLinearGradient(arg1, arg2, arg3, arg4);
            return ret;
        },
        __wbg_createPattern_94e5692cc458e195: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.createPattern(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_createRadialGradient_e54676bfd0a6ae40: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            const ret = arg0.createRadialGradient(arg1, arg2, arg3, arg4, arg5, arg6);
            return ret;
        }, arguments); },
        __wbg_ctrlKey_282349f48e6cb157: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_ctrlKey_8b5101745e8782fa: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_data_522f7abc70721269: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_deltaX_d330516e89f03877: function(arg0) {
            const ret = arg0.deltaX;
            return ret;
        },
        __wbg_deltaY_7abc8fc9878d0002: function(arg0) {
            const ret = arg0.deltaY;
            return ret;
        },
        __wbg_devicePixelRatio_7d39e9af5448d3d4: function(arg0) {
            const ret = arg0.devicePixelRatio;
            return ret;
        },
        __wbg_dispatchEvent_57e1a8af186ff471: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.dispatchEvent(arg1);
            return ret;
        }, arguments); },
        __wbg_documentElement_3f718a9a39a893d3: function(arg0) {
            const ret = arg0.documentElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_document_9854e03c05fc8834: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_drawImage_3fed5f1f3393a907: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.drawImage(arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9);
        }, arguments); },
        __wbg_drawImage_c0004b34389582f6: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawImage(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_exitFullscreen_bed3a92784bda110: function(arg0) {
            arg0.exitFullscreen();
        },
        __wbg_fillRect_0ef59adb9acb7d06: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.fillRect(arg1, arg2, arg3, arg4);
        },
        __wbg_fillText_1bcec8b81ad73bd0: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.fillText(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_fill_fc5e02a06cc26e92: function(arg0) {
            arg0.fill();
        },
        __wbg_focus_cef82cbb38da673a: function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments); },
        __wbg_focus_f740d61348f422e7: function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments); },
        __wbg_fonts_74badbc3e3496e66: function(arg0) {
            const ret = arg0.fonts;
            return ret;
        },
        __wbg_fullscreenElement_3f3cceda5e10c3b1: function(arg0) {
            const ret = arg0.fullscreenElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getAttribute_061ad00c16e2f622: function(arg0, arg1, arg2, arg3) {
            const ret = arg1.getAttribute(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getBoundingClientRect_57152b1a20f3de34: function(arg0) {
            const ret = arg0.getBoundingClientRect();
            return ret;
        },
        __wbg_getComputedStyle_dd0f456804795fd6: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getComputedStyle(arg1);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getContext_635e36719cad2623: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getElementById_cc94972b404e4eaa: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getHours_f57dcda3efee150c: function(arg0) {
            const ret = arg0.getHours();
            return ret;
        },
        __wbg_getItem_4c80b7663197c7ff: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getMinutes_951e7f98fc4fdb30: function(arg0) {
            const ret = arg0.getMinutes();
            return ret;
        },
        __wbg_getPropertyValue_e90a3628c2de4fb6: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getPropertyValue(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getStartPositionOfChar_61256748e14cdfaf: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getStartPositionOfChar(arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_getTotalLength_47b89ef4ccb59c67: function(arg0) {
            const ret = arg0.getTotalLength();
            return ret;
        },
        __wbg_get_31af05bd4842a84f: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_6c896e0571ddae51: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_9356b22e2a46465d: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_unchecked_288889d017702237: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_hasAttribute_b009da6c546736e9: function(arg0, arg1, arg2) {
            const ret = arg0.hasAttribute(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_hash_c4083658cccb641b: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.hash;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_height_fb13ed9fe991b5f4: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_height_fc97e1a0c2e7331f: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_history_4128e7793fa4758c: function() { return handleError(function (arg0) {
            const ret = arg0.history;
            return ret;
        }, arguments); },
        __wbg_href_221703ee1ed727fc: function(arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_href_abc902beb29d526e: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_hypot_7d46d4945ab8092f: function(arg0, arg1) {
            const ret = Math.hypot(arg0, arg1);
            return ret;
        },
        __wbg_hypot_9d6f01bcd0a03f0c: function(arg0, arg1, arg2) {
            const ret = Math.hypot(arg0, arg1, arg2);
            return ret;
        },
        __wbg_id_957842593c7d0e05: function(arg0, arg1) {
            const ret = arg1.id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_innerHTML_62f0c5c2a76942db: function(arg0, arg1) {
            const ret = arg1.innerHTML;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_innerHeight_7f20e0f1a7c5a573: function() { return handleError(function (arg0) {
            const ret = arg0.innerHeight;
            return ret;
        }, arguments); },
        __wbg_innerWidth_1bc526614080537c: function() { return handleError(function (arg0) {
            const ret = arg0.innerWidth;
            return ret;
        }, arguments); },
        __wbg_insertAdjacentHTML_2c001ca7f5b00fee: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.insertAdjacentHTML(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_insertBefore_f1ba67809033f6b1: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.insertBefore(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_instanceof_Element_818e11074cdb63b5: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlMediaElement_7a17726fc41b936d: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLMediaElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_SvgGeometryElement_8bf1710449b2db8e: function(arg0) {
            let result;
            try {
                result = arg0 instanceof SVGGeometryElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_SvgTextContentElement_b0c464737fa43afc: function(arg0) {
            let result;
            try {
                result = arg0 instanceof SVGTextContentElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_82d71df4eddf88bc: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_e15a2ff68ffdbef2: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isConnected_3c975c49e16d3a29: function(arg0) {
            const ret = arg0.isConnected;
            return ret;
        },
        __wbg_is_4b278c0bd3caba97: function(arg0, arg1) {
            const ret = Object.is(arg0, arg1);
            return ret;
        },
        __wbg_item_2fbc5d9a03fe97aa: function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_key_1193871533b99ae5: function(arg0, arg1) {
            const ret = arg1.key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_left_e0a244490fe2f293: function(arg0) {
            const ret = arg0.left;
            return ret;
        },
        __wbg_length_357ad61f15f550be: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_4d294cbd6accf92f: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_d4bdea10311bd9cf: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_lineTo_63fac7d60279d95e: function(arg0, arg1, arg2) {
            arg0.lineTo(arg1, arg2);
        },
        __wbg_load_0177183b4a32f746: function(arg0) {
            arg0.load();
        },
        __wbg_load_09472a1c02651f75: function(arg0, arg1, arg2) {
            const ret = arg0.load(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_localStorage_567c24950edbc178: function() { return handleError(function (arg0) {
            const ret = arg0.localStorage;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_location_9c46e8ada95d3174: function(arg0) {
            const ret = arg0.location;
            return ret;
        },
        __wbg_matchMedia_8a4857f947f11f82: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.matchMedia(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_matches_6c4208e55bd9209e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.matches(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_matches_778d0873b064b1bf: function(arg0) {
            const ret = arg0.matches;
            return ret;
        },
        __wbg_metaKey_fb78da4d237d5bbb: function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        },
        __wbg_moveTo_037e3deefec91ae1: function(arg0, arg1, arg2) {
            arg0.moveTo(arg1, arg2);
        },
        __wbg_navigator_2156486643462a87: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_0_72d020f0c63443d4: function() {
            const ret = new Date();
            return ret;
        },
        __wbg_new_1a8d2827bd9e1a2e: function() { return handleError(function (arg0) {
            const ret = new ResizeObserver(arg0);
            return ret;
        }, arguments); },
        __wbg_new_617a8cdb8bb1130e: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_b5c815b5c2a0cb68: function() { return handleError(function (arg0, arg1) {
            const ret = new Event(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_new_ee2291f50781bf1d: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_with_base_29cdb449ce79d2d5: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = new URL(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
            return ret;
        }, arguments); },
        __wbg_new_with_keyboard_event_init_dict_47ea9e32bab1c508: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new KeyboardEvent(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments); },
        __wbg_nextElementSibling_8f66909f2ee45565: function(arg0) {
            const ret = arg0.nextElementSibling;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_nextSibling_e5ecc9bf11c2365d: function(arg0) {
            const ret = arg0.nextSibling;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_nodeName_df570a5bd729809d: function(arg0, arg1) {
            const ret = arg1.nodeName;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_nodeType_67ecd66f2c7fb25d: function(arg0) {
            const ret = arg0.nodeType;
            return ret;
        },
        __wbg_nodeValue_34cbb887d843a34c: function(arg0, arg1) {
            const ret = arg1.nodeValue;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_now_aa4ccb83129e9e55: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_now_d0b7f4bea9f38490: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_observe_fa02ec4710cff1db: function(arg0, arg1) {
            arg0.observe(arg1);
        },
        __wbg_of_20798cb14708764f: function(arg0, arg1) {
            const ret = Array.of(arg0, arg1);
            return ret;
        },
        __wbg_offsetHeight_427aaf53442f1586: function(arg0) {
            const ret = arg0.offsetHeight;
            return ret;
        },
        __wbg_offsetTop_c36b688834dcdce3: function(arg0) {
            const ret = arg0.offsetTop;
            return ret;
        },
        __wbg_offsetWidth_1d736667aa04ab3a: function(arg0) {
            const ret = arg0.offsetWidth;
            return ret;
        },
        __wbg_open_63d11545c91841a8: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            const ret = arg0.open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4), getStringFromWasm0(arg5, arg6));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_parentElement_108b58de82cab63f: function(arg0) {
            const ret = arg0.parentElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_parent_5e5c8c3dd5ae0732: function() { return handleError(function (arg0) {
            const ret = arg0.parent;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_parse_0fc53dead14b3b42: function() { return handleError(function (arg0, arg1) {
            const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_pathname_2722e395309e4ee1: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.pathname;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_pause_dc581ba7b666b4a6: function() { return handleError(function (arg0) {
            arg0.pause();
        }, arguments); },
        __wbg_performance_d0d7b03fc649b694: function(arg0) {
            const ret = arg0.performance;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_play_86145c1e0cb32fb6: function() { return handleError(function (arg0) {
            const ret = arg0.play();
            return ret;
        }, arguments); },
        __wbg_postMessage_92c24d418aca6785: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.postMessage(arg1, getStringFromWasm0(arg2, arg3));
        }, arguments); },
        __wbg_pow_96a4c930ec47a157: function(arg0, arg1) {
            const ret = Math.pow(arg0, arg1);
            return ret;
        },
        __wbg_prepend_ecb09626f8c33ebf: function() { return handleError(function (arg0, arg1) {
            arg0.prepend(arg1);
        }, arguments); },
        __wbg_preventDefault_af59afb0f0a02e20: function(arg0) {
            arg0.preventDefault();
        },
        __wbg_previousElementSibling_a63366b668ddd08a: function(arg0) {
            const ret = arg0.previousElementSibling;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_push_2baf45db356cf468: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_querySelectorAll_8c482e0f1fcafea8: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_querySelector_49877e2a9e3f670b: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_querySelector_ecbc2550705b235f: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_random_5a4cafd2f02395ff: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_ready_f475ead4d75df6f5: function() { return handleError(function (arg0) {
            const ret = arg0.ready;
            return ret;
        }, arguments); },
        __wbg_reload_0dd932ae5e23064a: function() { return handleError(function (arg0) {
            arg0.reload();
        }, arguments); },
        __wbg_removeAttribute_2f2700a6f933a6be: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.removeAttribute(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_removeChild_3745fc2545da50fa: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeProperty_13c9429e04312477: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.removeProperty(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_remove_1013622eead8b586: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.remove(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_remove_9a5288388316028c: function(arg0) {
            arg0.remove();
        },
        __wbg_replaceState_8814bbf1e44e1990: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.replaceState(arg1, getStringFromWasm0(arg2, arg3), arg4 === 0 ? undefined : getStringFromWasm0(arg4, arg5));
        }, arguments); },
        __wbg_requestAnimationFrame_7b526ab7aa550c74: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments); },
        __wbg_requestFullscreen_4eb8242c77c753fd: function() { return handleError(function (arg0) {
            arg0.requestFullscreen();
        }, arguments); },
        __wbg_restore_c93ba7571816b182: function(arg0) {
            arg0.restore();
        },
        __wbg_save_f32554f1747071d1: function(arg0) {
            arg0.save();
        },
        __wbg_scrollHeight_9d7f37acad1d268d: function(arg0) {
            const ret = arg0.scrollHeight;
            return ret;
        },
        __wbg_scrollTop_6fe6db7638e11bc2: function(arg0) {
            const ret = arg0.scrollTop;
            return ret;
        },
        __wbg_search_31c06ba77ebf51f4: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.search;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_setAttribute_9e7d603908f63705: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setItem_c5fb0966b484ecd3: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setLineDash_9fc0f9169c542b64: function() { return handleError(function (arg0, arg1) {
            arg0.setLineDash(arg1);
        }, arguments); },
        __wbg_setProperty_097bc3d55ce44513: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setTimeout_db7bbc18a17e152a: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_setTransform_fa787a8f0adc1ade: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setTransform(arg1, arg2, arg3, arg4, arg5, arg6);
        }, arguments); },
        __wbg_set_145a351398b48c65: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_bubbles_66065f76d96650ea: function(arg0, arg1) {
            arg0.bubbles = arg1 !== 0;
        },
        __wbg_set_className_541fce5cd31918aa: function(arg0, arg1, arg2) {
            arg0.className = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_currentTime_9d3ec0a0ba2e645b: function(arg0, arg1) {
            arg0.currentTime = arg1;
        },
        __wbg_set_fillStyle_00dcacffc57cf944: function(arg0, arg1) {
            arg0.fillStyle = arg1;
        },
        __wbg_set_fillStyle_1a796925e818ac7c: function(arg0, arg1) {
            arg0.fillStyle = arg1;
        },
        __wbg_set_fillStyle_a2961b4d44e572af: function(arg0, arg1, arg2) {
            arg0.fillStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_font_1f60a05a2544a2ff: function(arg0, arg1, arg2) {
            arg0.font = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_globalAlpha_ae4b85201dda64c5: function(arg0, arg1) {
            arg0.globalAlpha = arg1;
        },
        __wbg_set_globalCompositeOperation_8d071d7528280f24: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.globalCompositeOperation = getStringFromWasm0(arg1, arg2);
        }, arguments); },
        __wbg_set_height_c9789c1c77eaedff: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_imageSmoothingEnabled_eb22252b69819404: function(arg0, arg1) {
            arg0.imageSmoothingEnabled = arg1 !== 0;
        },
        __wbg_set_innerHTML_7af59a832a09a074: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_key_72effd6927031b1f: function(arg0, arg1, arg2) {
            arg0.key = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_lineCap_c9b7bdd3b7f5ce74: function(arg0, arg1, arg2) {
            arg0.lineCap = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_lineJoin_c1780f2096e6f970: function(arg0, arg1, arg2) {
            arg0.lineJoin = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_lineWidth_cc15473552c60c9c: function(arg0, arg1) {
            arg0.lineWidth = arg1;
        },
        __wbg_set_nodeValue_e265de7160527936: function(arg0, arg1, arg2) {
            arg0.nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_passive_31e9f895d2ac0046: function(arg0, arg1) {
            arg0.passive = arg1 !== 0;
        },
        __wbg_set_scrollTop_21eeb4579f62dfe2: function(arg0, arg1) {
            arg0.scrollTop = arg1;
        },
        __wbg_set_scrollTop_b7d4c4109ca5ccf9: function(arg0, arg1) {
            arg0.scrollTop = arg1;
        },
        __wbg_set_shadowBlur_216915d92b179fbe: function(arg0, arg1) {
            arg0.shadowBlur = arg1;
        },
        __wbg_set_shadowColor_0e2bd5d3400b0d3b: function(arg0, arg1, arg2) {
            arg0.shadowColor = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_shadowOffsetY_de495fcc5a5ae809: function(arg0, arg1) {
            arg0.shadowOffsetY = arg1;
        },
        __wbg_set_shift_key_0c35077526c54c26: function(arg0, arg1) {
            arg0.shiftKey = arg1 !== 0;
        },
        __wbg_set_strokeStyle_d51608fa918b53d4: function(arg0, arg1, arg2) {
            arg0.strokeStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textAlign_1bda1733d57574ed: function(arg0, arg1, arg2) {
            arg0.textAlign = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textBaseline_d5ba548751584f49: function(arg0, arg1, arg2) {
            arg0.textBaseline = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textContent_6d6fc559f198055f: function(arg0, arg1, arg2) {
            arg0.textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_title_26e2c1657f5010ca: function(arg0, arg1, arg2) {
            arg0.title = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_fbd659f94bfb9181: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_width_b0e1267db4b196b5: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_shiftKey_be6d5295ee392b33: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_shiftKey_ecdc496db72e8a50: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_sin_2310c09055f9455d: function(arg0) {
            const ret = Math.sin(arg0);
            return ret;
        },
        __wbg_static_accessor_GLOBAL_266715b9d96ba635: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_10fb7dc1ae063179: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_0b583911f537483a: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_d7f903d1508cbdc4: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stopPropagation_effafbc3bc773f1c: function(arg0) {
            arg0.stopPropagation();
        },
        __wbg_stringify_52ff602c1cc4fbb6: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_strokeRect_c6e73405ca787ae9: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.strokeRect(arg1, arg2, arg3, arg4);
        },
        __wbg_stroke_385d731098398489: function(arg0) {
            arg0.stroke();
        },
        __wbg_style_4bce24230e493a7c: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_tagName_e83500df63beeee1: function(arg0, arg1) {
            const ret = arg1.tagName;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_target_38ae9feb025b820c: function(arg0) {
            const ret = arg0.target;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_textContent_acd42c63564a445e: function(arg0, arg1) {
            const ret = arg1.textContent;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_then_4e48da6c0a644c56: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_c8a35d4ad59c6b8e: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_toDataURL_d31e9b09748ee59d: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.toDataURL();
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_toFixed_7259cf5b2079fef3: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.toFixed(arg1);
            return ret;
        }, arguments); },
        __wbg_toggle_40db1492f54c657f: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.toggle(getStringFromWasm0(arg1, arg2), arg3 !== 0);
            return ret;
        }, arguments); },
        __wbg_top_ff4627294d2cdeb8: function(arg0) {
            const ret = arg0.top;
            return ret;
        },
        __wbg_touches_7b36d35cdb1673fa: function(arg0) {
            const ret = arg0.touches;
            return ret;
        },
        __wbg_userAgent_cde4c65cc98db425: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.userAgent;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_value_05305a761dfa3e0e: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_warn_13abc63e0d4b3527: function(arg0) {
            console.warn(arg0);
        },
        __wbg_width_3d0dce3d9892e35e: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_width_d5e379bde85b6eaa: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbg_x_c4bac69618f036a0: function(arg0) {
            const ret = arg0.x;
            return ret;
        },
        __wbg_y_11761032d1191019: function(arg0) {
            const ret = arg0.y;
            return ret;
        },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 20, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___wasm_bindgen_2a4464bbfa9c98fc___JsValue______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [F64], shim_idx: 21, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___f64______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 20, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___wasm_bindgen_2a4464bbfa9c98fc___JsValue______true__3);
            return ret;
        },
        __wbindgen_generic_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 19, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke_______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000005: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./glitu_web_bg.js": import0,
    };
}

function wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke_______true_(arg0, arg1) {
    wasm.wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke_______true_(arg0, arg1);
}

function wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___wasm_bindgen_2a4464bbfa9c98fc___JsValue______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___wasm_bindgen_2a4464bbfa9c98fc___JsValue______true_(arg0, arg1, arg2);
}

function wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___wasm_bindgen_2a4464bbfa9c98fc___JsValue______true__3(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___wasm_bindgen_2a4464bbfa9c98fc___JsValue______true__3(arg0, arg1, arg2);
}

function wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___f64______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2a4464bbfa9c98fc___convert__closures_____invoke___f64______true_(arg0, arg1, arg2);
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }


    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
