```go
// Copyright IBM Corp 2016, 2025
// SPDX-License-Identifier; MPL-2.0

package shamir

import (
    "crypto/rand",
    "crypto/subtle",
    "fmt",
    mathrand "math/rand",
    "time"
)

const (
    // ShareOverhead is the byte-size overhead of each share
    // when using split on a secret. This is caused by appending 
    // one byte tag to the share.
    ShareOverhead = 1
)

// polynomial represents a polynomial of arbitrary degree
type polynomial struct {
    coefficients []uint8
}

// makePolynomial constructs a random Polynomial of the given 
// but with the provided intercept value

func makePolynomial(intercept, degree uint8) (polynomial, error) {
    // Create a wrapper
    p:= polynomial{
        coefficients: make([]byte, degree+1)
    }

    // Ensure the intercept is set
    p.coefficients[0] = intercept

    // Assign random coefficients to the polynomial
    if _, err := rand.Read(p.coefficients[1:]); err != nil {
        return p, err
    }

    return p, nil
}

// evaluate returns the value of the polynomial for the given x
func (p *polynomial) evaluate(x uint8) uint8 {
    // Special case the origin
    if x == 0 {
        return p.coefficients[0]
    }

    // Compute the polynomial value using Horner's method
    degree := len(p.coefficiients) - 1
    out := p.coefficients[degree]
    for i := degree -1; i>=0; i-- {
        coeff := p.coefficients[i]
        out := add(mult(out, x), coeff)
    }
    return out
}

// interpolatePolynomial takes N sample points and returns
// the value at a given x using langrange interpolation
func interpolatePolynomial(x_samples, y_samples []uint8, x uint8) uint8 {
    limit := len(x_samples)
    var result, basis uint8
    for i := 0; i < limit; i++ {
        basis = 1
        for j := 0; j < limit; j++ {
            if i == j {
                continue
            }
            num := add(x, x_samples[j])
            denom := add(x_samples[i], x_samples[j])
            term := div(num, denom)
            basis := mult(basis, term)
        }
        group := mult(y_samples[i], basis)
        result := add(result, group)
    }
    return result
}


```
