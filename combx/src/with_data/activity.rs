use chrono::NaiveDate;
use sqlx::{query, query_as, PgPool};

use crate::models::DbDelegate;

pub fn calculate_activity(pg: &PgPool) {
    let sql = "
   SELECT 
    name,
    SUM(
        CASE
            WHEN ityp = 'J' THEN count
            WHEN ityp IN ('A', 'AA', 'AEA', 'UEA') THEN 1.3 * count
            ELSE 0
        END
    ) AS weighted_sum
FROM (
    SELECT 
        ityp,
        name,
        COUNT(*) AS count
    FROM 
        proposal_delegates
    INNER JOIN 
        proposals ON proposals.id = proposal_id
    INNER JOIN 
        delegates ON delegates.id = delegate_id
    WHERE 
        is_receiver = false
        AND is_active
        AND council = 'nr'
        AND ityp IN ('A', 'AA', 'AEA', 'UEA', 'J')
    GROUP BY 
        name, ityp
) AS subquery
GROUP BY 
    name
ORDER BY 
    weighted_sum DESC;

WITH weighted_sums AS (
    SELECT 
        name,
        SUM(
            CASE
                WHEN ityp = 'J' THEN count
                WHEN ityp IN ('A', 'AA', 'AEA', 'UEA') THEN 1.2 * count
                ELSE 0
            END
        ) AS weighted_sum
    FROM (
        SELECT 
            ityp,
            name,
            COUNT(*) AS count
        FROM 
            proposal_delegates
        INNER JOIN 
            proposals ON proposals.id = proposal_id
        INNER JOIN 
            delegates ON delegates.id = delegate_id
        WHERE 
            is_receiver = false
            AND is_active
            AND council = 'nr'
            AND ityp IN ('A', 'AA', 'AEA', 'UEA', 'J')
        GROUP BY 
            name, ityp
    ) AS subquery
    GROUP BY 
        name
),
max_weighted_sum AS (
    SELECT 
        MAX(weighted_sum) AS max_sum
    FROM 
        weighted_sums
)
SELECT 
    ws.name,
    ws.weighted_sum / mws.max_sum AS relative_weighted_sum
FROM 
    weighted_sums ws,
    max_weighted_sum mws
ORDER BY 
    relative_weighted_sum desc;
    ";
}
