class Matrix:
    def __init__(self, data):
        if isinstance(data, (int, float)):
            self.data = [[float(data)]]
            self.rows, self.cols = 1, 1
        elif isinstance(data, list) and len(data) > 0 and isinstance(data[0], (int, float)):
            self.data = [[float(x) for x in data]]
            self.rows, self.cols = 1, len(data)
        elif isinstance(data, list) and len(data) > 0 and isinstance(data[0], list):
            self.data = data
            self.rows = len(data)
            self.cols = len(data[0]) if self.rows > 0 else 0
        else:
            self.data = []
            self.rows, self.cols = 0, 0

    @classmethod
    def zeros(cls, rows: int, cols: int):
        return cls([[0.0 for _ in range(cols)] for _ in range(rows)])

    def __add__(self, other):
        if isinstance(other, (int, float)):
            return Matrix([[self.data[i][j] + other for j in range(self.cols)] for i in range(self.rows)])
        if self.rows != other.rows or self.cols != other.cols:
            raise ValueError(f"Addition dimension mismatch: {self.rows}x{self.cols} vs {other.rows}x{other.cols}")
        return Matrix([[self.data[i][j] + other.data[i][j] for j in range(self.cols)] for i in range(self.rows)])

    def __sub__(self, other):
        if isinstance(other, (int, float)):
            return Matrix([[self.data[i][j] - other for j in range(self.cols)] for i in range(self.rows)])
        if self.rows != other.rows or self.cols != other.cols:
            raise ValueError("Subtraction dimension mismatch.")
        return Matrix([[self.data[i][j] - other.data[i][j] for j in range(self.cols)] for i in range(self.rows)])

    def __mul__(self, scalar: float):
        """Element-wise scalar multiplication mapped to the * operator"""
        return Matrix([[self.data[i][j] * scalar for j in range(self.cols)] for i in range(self.rows)])

    def dot(self, other):
        """Matrix dot product execution"""
        if self.cols != other.rows:
            raise ValueError(f"Dot product dimension mismatch: {self.cols} cols vs {other.rows} rows.")
        C = Matrix.zeros(self.rows, other.cols)
        for i in range(self.rows):
            for j in range(other.cols):
                for k in range(self.cols):
                    C.data[i][j] += self.data[i][k] * other.data[k][j]
        return C

    def transpose(self):
        return Matrix([[self.data[j][i] for j in range(self.rows)] for i in range(self.cols)])

    def __eq__(self, other):
        return isinstance(other, Matrix) and self.data == other.data
