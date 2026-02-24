CREATE SCHEMA IF NOT EXISTS app;

CREATE TABLE IF NOT EXISTS app.customers (
    id              SERIAL PRIMARY KEY,
    first_name      VARCHAR(100) NOT NULL,
    last_name       VARCHAR(100) NOT NULL,
    email           VARCHAR(255) UNIQUE NOT NULL,
    phone           VARCHAR(50),
    industry_sector VARCHAR(100),   -- e.g., Data Science, Finance, Healthcare, IT
    account_number  VARCHAR(30) UNIQUE NOT NULL,
    balance         DOUBLE PRECISION  DEFAULT 0.00,
    currency        CHAR(3) DEFAULT 'CAD',
    credit_score    INTEGER,
    risk_level      VARCHAR(20),    -- low, medium, high
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO app.customers
(
    first_name,
    last_name,
    email,
    phone,
    industry_sector,
    account_number,
    balance,
    currency,
    credit_score,
    risk_level
)
VALUES

-- 1. Tony (Main Demo)
(
    'Tony',
    'Yousefnezhad',
    'tony@mail.com',
    '+1-780-555-1001',
    'Data Science',
    'ACC-200001',
    54230.75,
    'CAD',
    810,
    'low'
),

-- 2
('Sarah','Mitchell','sarah.m@mail.com','+1-416-555-2233','Marketing','ACC-200002',1340.50,'CAD',690,'medium'),

-- 3
('Michael','Chen','mchen@mail.com','+1-604-555-8899','Finance','ACC-200003',89200.00,'CAD',825,'low'),

-- 4
('Amina','Khan','amina.k@mail.com','+1-403-555-7788','Healthcare','ACC-200004',-540.25,'CAD',610,'high'),

-- 5
('Robert','Williams','rob.w@mail.com','+1-905-555-6677','Construction','ACC-200005',4500.00,'CAD',705,'medium'),

-- 6
('Emily','Brown','emily.b@mail.com','+1-250-555-9911','Education','ACC-200006',9800.00,'CAD',740,'low'),

-- 7
('David','Park','dpark@mail.com','+1-778-555-3322','IT Services','ACC-200007',17600.00,'CAD',760,'low'),

-- 8
('Fatima','Ali','fatima.a@mail.com','+1-647-555-8890','Retail','ACC-200008',2200.00,'CAD',680,'medium'),

-- 9
('James','Anderson','j.and@mail.com','+1-613-555-7766','Legal','ACC-200009',64000.00,'CAD',790,'low'),

-- 10
('Sofia','Garcia','sofia.g@mail.com','+1-514-555-4455','Hospitality','ACC-200010',3500.00,'CAD',695,'medium'),

-- 11
('Omar','Hassan','omar.h@mail.com','+1-780-555-6678','Logistics','ACC-200011',9100.00,'CAD',720,'low'),

-- 12
('Linda','Moore','linda.m@mail.com','+1-289-555-2234','HR','ACC-200012',12500.00,'CAD',750,'low'),

-- 13
('Kevin','Turner','kevin.t@mail.com','+1-905-555-3399','Manufacturing','ACC-200013',28400.00,'CAD',735,'medium'),

-- 14
('Nina','Petrov','nina.p@mail.com','+1-604-555-7781','Biotech','ACC-200014',41500.00,'CAD',805,'low'),

-- 15
('Yusuf','Demir','yusuf.d@mail.com','+1-647-555-9001','Energy','ACC-200015',540.00,'CAD',650,'medium'),

-- 16
('Rachel','Green','r.green@mail.com','+1-416-555-1122','Media','ACC-200016',18900.00,'CAD',770,'low'),

-- 17
('Wei','Zhang','wei.z@mail.com','+1-778-555-2244','AI Research','ACC-200017',73000.00,'CAD',830,'low'),

-- 18
('Carlos','Mendez','c.m@mail.com','+1-587-555-3321','Transportation','ACC-200018',4600.00,'CAD',700,'medium'),

-- 19
('Hannah','Scott','h.scott@mail.com','+1-902-555-7789','Public Sector','ACC-200019',15200.00,'CAD',745,'low'),

-- 20
('Ivan','Kozlov','ivan.k@mail.com','+1-780-555-4433','Cybersecurity','ACC-200020',92000.00,'CAD',840,'low');
