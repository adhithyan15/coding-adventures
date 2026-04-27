#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub data: Vec<Vec<f64>>,
    pub rows: usize,
    pub cols: usize,
}

impl Matrix {
    pub fn new_2d(data: Vec<Vec<f64>>) -> Self {
        let rows = data.len();
        let cols = if rows > 0 { data[0].len() } else { 0 };
        Self { data, rows, cols }
    }

    pub fn new_1d(data: Vec<f64>) -> Self {
        let cols = data.len();
        Self { data: vec![data], rows: 1, cols }
    }

    pub fn new_scalar(val: f64) -> Self {
        Self { data: vec![vec![val]], rows: 1, cols: 1 }
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self { data: vec![vec![0.0; cols]; rows], rows, cols }
    }

    pub fn add(&self, other: &Matrix) -> Result<Self, &'static str> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err("Matrix addition dimensions rigorously mismatch");
        }
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] + other.data[i][j];
            }
        }
        Ok(c)
    }
    
    pub fn add_scalar(&self, scalar: f64) -> Self {
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] + scalar;
            }
        }
        c
    }

    pub fn subtract(&self, other: &Matrix) -> Result<Self, &'static str> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err("Matrix subtraction dimensions rigorously mismatch");
        }
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] - other.data[i][j];
            }
        }
        Ok(c)
    }

    pub fn scale(&self, scalar: f64) -> Self {
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] * scalar;
            }
        }
        c
    }

    pub fn transpose(&self) -> Self {
        if self.rows == 0 { return Self::zeros(0, 0); }
        let mut c = Self::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[j][i] = self.data[i][j];
            }
        }
        c
    }

    pub fn dot(&self, other: &Matrix) -> Result<Self, &'static str> {
        if self.cols != other.rows {
            return Err("Matrix dot mapping inner dimensions strictly contradict");
        }
        let mut c = Self::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            for j in 0..other.cols {
                for k in 0..self.cols {
                    c.data[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }
        Ok(c)
    }
}
