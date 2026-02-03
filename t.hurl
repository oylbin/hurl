GET http://localhost:57001/section/list
HTTP 200
[Captures]
section_count: jsonpath "$.data.sections" count
last_section_id: variable "section_count" toInt
new_section_id: variable "last_section_id" toInt jsfilter add 1
section1: jsonpath "$.data.sections" nth 0
section1str: jsonpath "$.data.sections" nth 0 jsfilter jsonstr
sectionsign: variable "section1" jsfilter sign_request "12345" jsfilter jsonstr