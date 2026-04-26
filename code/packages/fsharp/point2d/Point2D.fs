namespace CodingAdventures.Point2D

open System

[<Struct>]
type Point =
    {
        X: float
        Y: float
    }

    static member New(x: float, y: float) = { X = x; Y = y }
    static member Origin() = Point.New(0.0, 0.0)

    member this.Add(other: Point) = Point.New(this.X + other.X, this.Y + other.Y)
    member this.Subtract(other: Point) = Point.New(this.X - other.X, this.Y - other.Y)
    member this.Scale(scalar: float) = Point.New(this.X * scalar, this.Y * scalar)
    member this.Negate() = Point.New(-this.X, -this.Y)
    member this.Dot(other: Point) = this.X * other.X + this.Y * other.Y
    member this.Cross(other: Point) = this.X * other.Y - this.Y * other.X
    member this.MagnitudeSquared() = this.X * this.X + this.Y * this.Y
    member this.Magnitude() = Math.Sqrt(this.MagnitudeSquared())

    member this.Normalize() =
        let magnitude = this.Magnitude()
        if magnitude < 1e-12 then Point.Origin() else Point.New(this.X / magnitude, this.Y / magnitude)

    member this.Distance(other: Point) = this.Subtract(other).Magnitude()
    member this.DistanceSquared(other: Point) = this.Subtract(other).MagnitudeSquared()
    member this.Lerp(other: Point, t: float) = Point.New(this.X + (other.X - this.X) * t, this.Y + (other.Y - this.Y) * t)
    member this.Perpendicular() = Point.New(-this.Y, this.X)
    member this.Angle() = Math.Atan2(this.Y, this.X)

[<Struct>]
type Rect =
    {
        X: float
        Y: float
        Width: float
        Height: float
    }

    static member New(x: float, y: float, width: float, height: float) =
        { X = x; Y = y; Width = width; Height = height }

    static member Zero() = Rect.New(0.0, 0.0, 0.0, 0.0)
    static member FromPoints(minimum: Point, maximum: Point) =
        Rect.New(minimum.X, minimum.Y, maximum.X - minimum.X, maximum.Y - minimum.Y)

    member this.MinPoint() = Point.New(this.X, this.Y)
    member this.MaxPoint() = Point.New(this.X + this.Width, this.Y + this.Height)
    member this.Center() = Point.New(this.X + this.Width * 0.5, this.Y + this.Height * 0.5)
    member this.IsEmpty() = this.Width <= 0.0 || this.Height <= 0.0

    member this.ContainsPoint(point: Point) =
        not (this.IsEmpty())
        && point.X >= this.X
        && point.Y >= this.Y
        && point.X < this.X + this.Width
        && point.Y < this.Y + this.Height

    member this.Union(other: Rect) =
        if this.IsEmpty() then
            other
        elif other.IsEmpty() then
            this
        else
            let minX = min this.X other.X
            let minY = min this.Y other.Y
            let maxX = max (this.X + this.Width) (other.X + other.Width)
            let maxY = max (this.Y + this.Height) (other.Y + other.Height)
            Rect.New(minX, minY, maxX - minX, maxY - minY)

    member this.Intersection(other: Rect) =
        let minX = max this.X other.X
        let minY = max this.Y other.Y
        let maxX = min (this.X + this.Width) (other.X + other.Width)
        let maxY = min (this.Y + this.Height) (other.Y + other.Height)
        let width = maxX - minX
        let height = maxY - minY
        if width <= 0.0 || height <= 0.0 then None else Some(Rect.New(minX, minY, width, height))

    member this.ExpandBy(amount: float) =
        Rect.New(this.X - amount, this.Y - amount, this.Width + 2.0 * amount, this.Height + 2.0 * amount)
