use std::arch::x86_64::{_mm256_fmadd_ps, _mm_rcp_ps, _mm_set_ps, _mm_storeu_ps};
use std::mem::transmute;
use ultraviolet::{f32x8, Vec3, Vec3x8};
#[inline(always)]
fn vfmadd(a: f32x8, b: f32x8, c: f32x8) -> f32x8 {
    #[cfg(all(target_feature="avx", target_feature = "fma"))]
    {
        unsafe {
            return transmute(_mm256_fmadd_ps(transmute(a), transmute(b), transmute(c)));
        }
    }

    #[cfg(all(not(target_feature="avx"), target_feature = "sse", target_feature = "fma"))]
    unsafe {
        use core::arch::x86_64::*;
        #[repr(C)]
        struct F32x8Split {
            lower: __m128,
            upper: __m128,
        }

        let a_split: F32x8Split = transmute(a);
        let b_split: F32x8Split = transmute(b);
        let c_split: F32x8Split = transmute(c);

        let result_lower = _mm_fmadd_ps(a_split.lower, b_split.lower, c_split.lower);
        let result_upper = _mm_fmadd_ps(a_split.upper, b_split.upper, c_split.upper);

        return transmute(F32x8Split {
            lower: result_lower,
            upper: result_upper,
        });
    }

    a * b - c
}
#[inline(always)]
pub fn vecmuladd(a: Vec3x8, b: Vec3x8, c: Vec3x8) -> Vec3x8 {
    Vec3x8::new(vfmadd(a.x, b.x, c.x), vfmadd(a.y, b.y, c.y), vfmadd(a.z, b.z, c.z),)
}


#[inline]
pub fn fast_vec3_recip(v: Vec3) -> Vec3 {
    #[cfg(target_feature = "sse")]
    {
        unsafe { fast_vec3_recip_sse(v) }
    }

    #[cfg(not(target_feature = "sse"))]
    {
        fast_vec3_recip_fallback(v)
    }
}

#[inline]
#[target_feature(enable = "sse")]
unsafe fn fast_vec3_recip_sse(v: Vec3) -> Vec3 {
    unsafe {
        let packed = _mm_set_ps(0.0, v.z, v.y, v.x);

        let recip = _mm_rcp_ps(packed);

        let mut result = [0.0f32; 4];
        _mm_storeu_ps(result.as_mut_ptr(), recip);

        Vec3::new(result[0], result[1], result[2])
    }
}

#[inline]
fn fast_vec3_recip_fallback(v: Vec3) -> Vec3 {
    Vec3::one() / v
}