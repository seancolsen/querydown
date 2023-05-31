export const starting_querydown = `issues
created_at:<@6M|ago
due_date:<@now
description|length:>200
--#assignments
++#labels{name:["Regression" "Bug"]}
#comments{user.team.name!"Backend"}:10
$id
$author.username -> author
$#comments.created_at%min -> first_comment_date \\sd
`;