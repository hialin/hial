# Models of various domains

## File system

dir
    file
        @size
        @modification_time
        @access_time
        @creation_time
        @owner
        @group
        @permissions

## JSON

object
    key: value
    key: array
        value
        object
    key: object

# xml

xml
    @version
    @encoding
    @standalone
    doctype
    pi
    element
        @attr1
        @attr2
        text
        element1
        element2
        comment
        cdata

# http

response
    @status_code
    @status_text
    @headers
    @body
