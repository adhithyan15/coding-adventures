namespace CodingAdventures.Point2D.Tests

open System
open CodingAdventures.Point2D
open Xunit

type Point2DTests() =
    [<Fact>]
    member _.PointArithmeticAndVectorOperationsWork() =
        let point = Point.New(3.0, 4.0)
        let other = Point.New(1.0, 2.0)

        Assert.Equal(Point.New(4.0, 6.0), point.Add other)
        Assert.Equal(Point.New(2.0, 2.0), point.Subtract other)
        Assert.Equal(Point.New(6.0, 8.0), point.Scale 2.0)
        Assert.Equal(Point.New(-3.0, -4.0), point.Negate())
        Assert.Equal(11.0, point.Dot other, 9)
        Assert.Equal(2.0, point.Cross other, 9)
        Assert.Equal(5.0, point.Magnitude(), 9)
        Assert.Equal(25.0, point.MagnitudeSquared(), 9)

    [<Fact>]
    member _.PointDerivedOperationsWork() =
        let point = Point.New(3.0, 4.0)

        Assert.Equal(Point.New(0.0, 0.0), Point.Origin())
        Assert.Equal(Point.New(0.6, 0.8), point.Normalize())
        Assert.Equal(Point.Origin(), Point.Origin().Normalize())
        Assert.Equal(5.0, point.Distance(Point.Origin()), 9)
        Assert.Equal(25.0, point.DistanceSquared(Point.Origin()), 9)
        Assert.Equal(Point.New(5.0, 5.0), Point.Origin().Lerp(Point.New(10.0, 10.0), 0.5))
        Assert.Equal(Point.New(-4.0, 3.0), point.Perpendicular())
        Assert.Equal(Math.Atan2(4.0, 3.0), point.Angle(), 9)

    [<Fact>]
    member _.RectAccessorsAndContainmentWork() =
        let rect = Rect.New(10.0, 20.0, 30.0, 40.0)

        Assert.Equal(Rect.Zero(), Rect.New(0.0, 0.0, 0.0, 0.0))
        Assert.Equal(Rect.New(10.0, 20.0, 30.0, 40.0), Rect.FromPoints(Point.New(10.0, 20.0), Point.New(40.0, 60.0)))
        Assert.Equal(Point.New(10.0, 20.0), rect.MinPoint())
        Assert.Equal(Point.New(40.0, 60.0), rect.MaxPoint())
        Assert.Equal(Point.New(25.0, 40.0), rect.Center())
        Assert.False(rect.IsEmpty())
        Assert.True(Rect.New(0.0, 0.0, 0.0, 5.0).IsEmpty())
        Assert.True(rect.ContainsPoint(Point.New(10.0, 20.0)))
        Assert.False(rect.ContainsPoint(Point.New(40.0, 60.0)))

    [<Fact>]
    member _.RectUnionIntersectionAndExpansionWork() =
        let left = Rect.New(0.0, 0.0, 10.0, 10.0)
        let right = Rect.New(5.0, 5.0, 10.0, 10.0)

        Assert.Equal(Rect.New(0.0, 0.0, 15.0, 15.0), left.Union right)
        Assert.Equal(right, Rect.Zero().Union right)
        Assert.Equal(left, left.Union(Rect.Zero()))
        Assert.Equal(Some(Rect.New(5.0, 5.0, 5.0, 5.0)), left.Intersection right)
        Assert.Equal(None, left.Intersection(Rect.New(20.0, 20.0, 2.0, 2.0)))
        Assert.Equal(Rect.New(-2.0, -2.0, 14.0, 14.0), left.ExpandBy(2.0))
