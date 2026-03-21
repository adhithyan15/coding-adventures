use matrix::Matrix;

#[test]
fn test_zeros() {
    let z = Matrix::zeros(2, 3);
    assert_eq!(z.rows, 2);
    assert_eq!(z.cols, 3);
    assert_eq!(z.data[1][2], 0.0);
}

#[test]
fn test_add_subtract() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let b = Matrix::new_2d(vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
    
    let c = a.add(&b).unwrap();
    assert_eq!(c.data, vec![vec![6.0, 8.0], vec![10.0, 12.0]]);
    
    let d = b.subtract(&a).unwrap();
    assert_eq!(d.data, vec![vec![4.0, 4.0], vec![4.0, 4.0]]);
    
    // Scalar addition execution
    let e = a.add_scalar(2.0);
    assert_eq!(e.data, vec![vec![3.0, 4.0], vec![5.0, 6.0]]);
    
    assert!(a.add(&Matrix::new_scalar(1.0)).is_err());
}

#[test]
fn test_scale() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let c = a.scale(2.0);
    assert_eq!(c.data, vec![vec![2.0, 4.0], vec![6.0, 8.0]]);
}

#[test]
fn test_transpose() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    let c = a.transpose();
    assert_eq!(c.data, vec![vec![1.0, 4.0], vec![2.0, 5.0], vec![3.0, 6.0]]);
}

#[test]
fn test_dot() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let b = Matrix::new_2d(vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
    
    let c = a.dot(&b).unwrap();
    assert_eq!(c.data, vec![vec![19.0, 22.0], vec![43.0, 50.0]]);
    
    let d = Matrix::new_1d(vec![1.0, 2.0, 3.0]);
    let e = Matrix::new_2d(vec![vec![4.0], vec![5.0], vec![6.0]]);
    let f = d.dot(&e).unwrap();
    assert_eq!(f.data, vec![vec![32.0]]);
    
    assert!(a.dot(&e).is_err());
}
