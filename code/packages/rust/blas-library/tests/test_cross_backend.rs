//! Cross-backend consistency tests: verify all 7 backends produce identical results.
//!
//! These tests run the same operation on every backend and assert that the
//! results match the CPU reference exactly. This is the core quality guarantee.

use blas_library::traits::BlasBackend;
use blas_library::{BackendRegistry, Matrix, Side, Transpose, Vector};

fn all_backends() -> Vec<(String, Box<dyn BlasBackend>)> {
    let registry = BackendRegistry::with_defaults();
    registry
        .list_available()
        .into_iter()
        .map(|name| {
            let backend = registry.get(&name).unwrap();
            (name, backend)
        })
        .collect()
}

// =========================================================================
// Level 1 cross-backend tests
// =========================================================================

#[test]
fn test_cross_saxpy() {
    let x = Vector::new(vec![1.0, -2.0, 3.0, -4.0, 5.0]);
    let y = Vector::new(vec![5.0, 4.0, -3.0, 2.0, -1.0]);
    let alpha = 2.5;

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.saxpy(alpha, &x, &y).unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SAXPY mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_sdot() {
    let x = Vector::new(vec![1.0, 2.0, 3.0, 4.0]);
    let y = Vector::new(vec![-1.0, 2.0, -3.0, 4.0]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.sdot(&x, &y).unwrap();
        if let Some(exp) = expected {
            assert_eq!(result, exp, "SDOT mismatch on {}", name);
        } else {
            expected = Some(result);
        }
    }
}

#[test]
fn test_cross_snrm2() {
    let x = Vector::new(vec![1.0, 2.0, 3.0, 4.0, 5.0]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.snrm2(&x);
        if let Some(exp) = expected {
            assert_eq!(result, exp, "SNRM2 mismatch on {}", name);
        } else {
            expected = Some(result);
        }
    }
}

#[test]
fn test_cross_sscal() {
    let x = Vector::new(vec![1.0, -2.0, 3.0]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.sscal(-3.0, &x);
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SSCAL mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_sasum() {
    let x = Vector::new(vec![-1.0, 2.0, -3.0, 4.0, -5.0]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.sasum(&x);
        if let Some(exp) = expected {
            assert_eq!(result, exp, "SASUM mismatch on {}", name);
        } else {
            expected = Some(result);
        }
    }
}

#[test]
fn test_cross_isamax() {
    let x = Vector::new(vec![1.0, -10.0, 5.0, -3.0]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.isamax(&x);
        if let Some(exp) = expected {
            assert_eq!(result, exp, "ISAMAX mismatch on {}", name);
        } else {
            expected = Some(result);
        }
    }
}

#[test]
fn test_cross_scopy() {
    let x = Vector::new(vec![3.14, 2.71, 1.41]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.scopy(&x);
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SCOPY mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_sswap() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);

    let mut expected_x = None;
    let mut expected_y = None;
    for (name, backend) in all_backends() {
        let (rx, ry) = backend.sswap(&x, &y).unwrap();
        if let Some(ref ex) = expected_x {
            assert_eq!(rx.data(), ex, "SSWAP X mismatch on {}", name);
            assert_eq!(
                ry.data(),
                expected_y.as_ref().unwrap(),
                "SSWAP Y mismatch on {}",
                name
            );
        } else {
            expected_x = Some(rx.data().to_vec());
            expected_y = Some(ry.data().to_vec());
        }
    }
}

// =========================================================================
// Level 2 cross-backend tests
// =========================================================================

#[test]
fn test_cross_sgemv() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![10.0, 20.0]);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend
            .sgemv(Transpose::NoTrans, 2.0, &a, &x, 1.0, &y)
            .unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SGEMV mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_sger() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0]);
    let a = Matrix::new(vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0], 3, 2);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.sger(2.0, &x, &y, &a).unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SGER mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

// =========================================================================
// Level 3 cross-backend tests
// =========================================================================

#[test]
fn test_cross_sgemm() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let b = Matrix::new(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], 3, 2);
    let c = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend
            .sgemm(Transpose::NoTrans, Transpose::NoTrans, 2.0, &a, &b, 0.5, &c)
            .unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SGEMM mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_sgemm_trans() {
    let a = Matrix::new(vec![1.0, 3.0, 2.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend
            .sgemm(Transpose::Trans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
            .unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SGEMM Trans mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_ssymm() {
    let a = Matrix::new(vec![2.0, 1.0, 1.0, 3.0], 2, 2);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(2, 2);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend.ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SSYMM mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}

#[test]
fn test_cross_sgemm_batched() {
    let a_list = vec![
        Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2),
        Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2),
    ];
    let b_list = vec![
        Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2),
        Matrix::new(vec![2.0, 0.0, 0.0, 2.0], 2, 2),
    ];
    let c_list = vec![Matrix::zeros(2, 2), Matrix::zeros(2, 2)];

    let mut expected: Option<Vec<Vec<f32>>> = None;
    for (name, backend) in all_backends() {
        let results = backend
            .sgemm_batched(
                Transpose::NoTrans,
                Transpose::NoTrans,
                1.0,
                &a_list,
                &b_list,
                0.0,
                &c_list,
            )
            .unwrap();
        if let Some(ref exp) = expected {
            for (i, r) in results.iter().enumerate() {
                assert_eq!(
                    r.data(),
                    exp[i].as_slice(),
                    "SGEMM_BATCHED[{}] mismatch on {}",
                    i,
                    name
                );
            }
        } else {
            expected = Some(results.iter().map(|r| r.data().to_vec()).collect::<Vec<_>>());
        }
    }
}

// =========================================================================
// Additional cross-backend tests for error cases
// =========================================================================

#[test]
fn test_cross_saxpy_error() {
    let x = Vector::new(vec![1.0]);
    let y = Vector::new(vec![1.0, 2.0]);
    for (name, backend) in all_backends() {
        assert!(
            backend.saxpy(1.0, &x, &y).is_err(),
            "{} should reject mismatched SAXPY",
            name
        );
    }
}

#[test]
fn test_cross_sdot_error() {
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![1.0]);
    for (name, backend) in all_backends() {
        assert!(
            backend.sdot(&x, &y).is_err(),
            "{} should reject mismatched SDOT",
            name
        );
    }
}

#[test]
fn test_cross_sswap_error() {
    let x = Vector::new(vec![1.0]);
    let y = Vector::new(vec![1.0, 2.0]);
    for (name, backend) in all_backends() {
        assert!(
            backend.sswap(&x, &y).is_err(),
            "{} should reject mismatched SSWAP",
            name
        );
    }
}

#[test]
fn test_cross_sgemm_error() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    for (name, backend) in all_backends() {
        assert!(
            backend
                .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
                .is_err(),
            "{} should reject mismatched SGEMM",
            name
        );
    }
}

#[test]
fn test_cross_sgemv_error() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::zeros(2);
    for (name, backend) in all_backends() {
        assert!(
            backend
                .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
                .is_err(),
            "{} should reject mismatched SGEMV",
            name
        );
    }
}

#[test]
fn test_cross_sger_error() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![1.0, 2.0]);
    let a = Matrix::zeros(2, 2);
    for (name, backend) in all_backends() {
        assert!(
            backend.sger(1.0, &x, &y, &a).is_err(),
            "{} should reject mismatched SGER",
            name
        );
    }
}

#[test]
fn test_cross_ssymm_not_square() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let b = Matrix::zeros(2, 2);
    let c = Matrix::zeros(2, 2);
    for (name, backend) in all_backends() {
        assert!(
            backend.ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).is_err(),
            "{} should reject non-square SSYMM",
            name
        );
    }
}

#[test]
fn test_cross_sgemm_batched_error() {
    let a = vec![Matrix::zeros(2, 2)];
    let b = vec![Matrix::zeros(2, 2), Matrix::zeros(2, 2)];
    let c = vec![Matrix::zeros(2, 2)];
    for (name, backend) in all_backends() {
        assert!(
            backend
                .sgemm_batched(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
                .is_err(),
            "{} should reject mismatched batched GEMM sizes",
            name
        );
    }
}

#[test]
fn test_cross_sgemv_transposed() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::zeros(3);

    let mut expected = None;
    for (name, backend) in all_backends() {
        let result = backend
            .sgemv(Transpose::Trans, 1.0, &a, &x, 0.0, &y)
            .unwrap();
        if let Some(ref exp) = expected {
            assert_eq!(result.data(), exp, "SGEMV Trans mismatch on {}", name);
        } else {
            expected = Some(result.data().to_vec());
        }
    }
}
