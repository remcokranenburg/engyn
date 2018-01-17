# Adaptive Quality

The effective quality level depends on three aspects: the global quality level, the user-
configurable-weight of a feature, and the relative cost function of the feature.

## Global quality level

Global quality level is determined by past performance.

## User-configurable weights


### Idea 1: Biasing a curve

The weights that are set by the user are interpreted as modifiers to the cost function of each
feature. They bend the linear scale towards the positive or the negative, maybe based on the curve
of a conic section, or a logistic function.

Criteria for weighting curve:

 * Curve is a function that takes global quality level as input (x) and a feature level as
   output (y)
 * Curve should be defined by one parameter: the weight.
 * Input is floating point value, but can be unbounded.
 * Output should be floating point value between 0 and 1.
 * When the weight is minimized, the output should still go to 1 as input (quality) is maximized
 * When the weight is maximized, the output should still go to 0 as input (quality) is minimized

### Idea 2: Duration-based weighting

Use the relative cost of each feature to calculate the approximate amount of time a unit of a
feature level will take during a frame. Then, use the weights relative to each other to determine
the desired distribution of frame time towards particular features. For continuously scaling feature
levels, the distribution can be precise. For discrete feature levels, a mismatch is expected and
can be filled by increasing other feature levels until the budget is filled.

## Relative cost of each feature

Rendering a scene has a cost and that cost is influenced by configurable features. To make correct
decisions regarding scaling back the impact level of a feature, it is necessary how the performance
is affected by scaling feature levels.

Some features are on a continuous scale, while others are on a discrete scale. Continuous features
can be sampled, while discrete features can be exhaustively tested. Since the Adaptive Quality
algorithm depends on parameters that have a continuous scale, we need to convert the discrete
functions into continuous functions.

OR, we don't convert the discrete scales to continuous scales, because the optimizer needs to know
about the discrete levels anyway to make the right decision.

The result of a performance test for a feature is a function with as input the feature level, and as
output the execution time. To turn this into a relative cost, on a normalized [0.0-1.0] scale, we
need to run performance tests for each feature in isolation and normalize in such a way that the
optimizer can create a scaling strategy that smoothly scales the execution time from lowest to
highest. This is especially challenging with discrete features, since such a level change causes a
discontinuous change in execution time, which needs to be compensated for by scaling other feature
levels.

Note: we assume that each feature fully independently influences performance, but they might be
affected by each other depending on shared resources (e.g. a limited amount of memory)
