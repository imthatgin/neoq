use neoq_macro::cypher;

#[test]
fn test_basic_query() {
    let query = cypher! {
        MATCH (user:User)
        RETURN user
    };
    assert_eq!(query, "MATCH (user:User)\nRETURN user\n")
}


#[test]
fn test_conditional_query() {
    let mut condition = false;
    let falsy_query = cypher! {
        MATCH (user:User)
        if condition {
            WHERE user.admin = true
        }
        RETURN user
    };
    assert_eq!(falsy_query, "MATCH (user:User)\nRETURN user\n");

    condition = true;
    let truthy_query = cypher! {
        MATCH (user:User)
        if condition {
            WHERE user.admin = true
        }
        RETURN user
    };
    assert_eq!(truthy_query, "MATCH (user:User)\nWHERE user.admin = true\nRETURN user\n");
}

#[test]
fn test_complex_query_example() {
    let query = cypher! {
        MATCH (p:Person)-[:HAS_INTEREST]->(interestNode) // Match a Person and their direct interest
        WHERE p.age >= 25 AND p.age <= 40 // Filter persons by age
        WITH p, interestNode // Pass through person and their interest
        MATCH (interestNode)-[:BOUGHT]->(product:Product) // Find products related to the interest
        WHERE product.category = "Electronics" // Filter products by category
        WITH p, COUNT(DISTINCT product) AS distinctProductsCount // Count distinct products for each person
        WHERE distinctProductsCount > 2 // Filter persons who are interested in more than 2 distinct products
        RETURN p.name AS PersonName, distinctProductsCount // Return the person's name and the count
        ORDER BY distinctProductsCount DESC // Order the results
        LIMIT 10 // Limit the results to the top 10
    };

    assert_eq!(query, "MATCH (p:Person)-[:HAS_INTEREST]->(interestNode)\nWHERE p.age >= 25 AND p.age <= 40\nWITH p, interestNode\nMATCH (interestNode)-[:BOUGHT]->(product:Product)\nWHERE product.category = \"Electronics\"\nWITH p, COUNT(DISTINCT product) AS distinctProductsCount\nWHERE distinctProductsCount > 2\nRETURN p.name AS PersonName, distinctProductsCount\nORDER BY distinctProductsCount DESC\nLIMIT 10\n");
}

#[test]
fn test_complex_return() {
    let query = cypher! {
        MATCH (p:Person) // Match a Person and their direct interest
        RETURN p {
            .*,
            .name,
            interests: [(p)-[:HAS_INTEREST]-(interest:Interest) | interest { .* }],
            posts: [(p)-[:POSTED]-(post:Post) | post { .* }]
        }
    };

    assert_eq!(query, "MATCH (p:Person)\nRETURN p {\n    .*, .name, interests:\n    [(p)-[:HAS_INTEREST]-(interest:Interest) | interest { .* }], posts:\n    [(p)-[:POSTED]-(post:Post) | post { .* }]\n}\n");
}